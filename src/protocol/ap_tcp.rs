//! AP 模式配网 TCP 服务：端口 1234，JSON 行协议 identify / config
//! 配网成功后保持 SoftAP，通过 channel 回传 success + deviceId/staIp/mac 给 Flutter，再继续读下一行。

use log::{info, warn};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::system::config::{AP_IP_ADDRESS, AP_TCP_PORT};
use crate::wifi::{WifiCommand, WifiResponse};

type WifiCmdTx = mpsc::Sender<(WifiCommand, mpsc::Sender<WifiResponse>)>;

/// 主循环在 STA 连接成功时向此 Sender 发送 (device_id, sta_ip, mac)，AP TCP 线程据此回传 Flutter（可选，新流程下不等待）
pub type PendingConfigDone = Arc<Mutex<Option<mpsc::Sender<(String, String, String)>>>>;

/// 配网时手机传来的 bindToken，主循环在 STA 连上后取走并传给 STA 服务，用于持续发 binding 直至收到回信
pub type PendingBindToken = Arc<Mutex<Option<String>>>;

/// 启动 AP 模式 TCP 监听（在独立线程中），处理 identify 与 config
pub fn start_ap_tcp_listener(
    device_id: String,
    fw_version: String,
    wifi_cmd_tx: WifiCmdTx,
    on_config_success: Option<mpsc::Sender<(String, String)>>,
    pending_config_done: PendingConfigDone,
    pending_bind_token: PendingBindToken,
) {
    thread::Builder::new()
        .name("ap-tcp-1234".into())
        .spawn(move || {
            // 必须绑定到 AP 的 IP，否则 lwIP 可能监听在错误网卡，手机连热点后无法连上
            let addr = format!("{}:{}", AP_IP_ADDRESS, AP_TCP_PORT);
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => l,
                Err(e) => {
                    warn!("AP TCP {} bind failed: {}", addr, e);
                    return;
                }
            };
            info!("AP TCP listener on {}", addr);
            for stream in listener.incoming().filter_map(Result::ok) {
                let device_id = device_id.clone();
                let fw_version = fw_version.clone();
                let wifi_cmd_tx = wifi_cmd_tx.clone();
                let on_config_success = on_config_success.clone();
                let pending_config_done = Arc::clone(&pending_config_done);
                let pending_bind_token = Arc::clone(&pending_bind_token);
                thread::spawn(move || {
                    handle_ap_client(
                        stream,
                        device_id,
                        fw_version,
                        wifi_cmd_tx,
                        on_config_success,
                        pending_config_done,
                        pending_bind_token,
                    );
                });
            }
        })
        .expect("spawn ap-tcp thread");
}

/// 从 stream 读一行（到 \n 或 EOF），不依赖 try_clone（ESP32 上 try_clone 可能不可用）
fn read_line(stream: &mut std::net::TcpStream) -> std::io::Result<String> {
    let mut line = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if buf[0] == b'\n' {
            break;
        }
        if buf[0] != b'\r' {
            line.push(buf[0]);
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

fn handle_ap_client(
    mut stream: std::net::TcpStream,
    device_id: String,
    fw_version: String,
    wifi_cmd_tx: WifiCmdTx,
    on_config_success: Option<mpsc::Sender<(String, String)>>,
    pending_config_done: PendingConfigDone,
    pending_bind_token: PendingBindToken,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    loop {
        let line = match read_line(&mut stream) {
            Ok(s) => s,
            Err(e) => {
                warn!("AP TCP read error: {}", e);
                break;
            }
        };
        let msg = line.trim();
        if msg.is_empty() {
            continue;
        }
        let json: serde_json::Value = match serde_json::from_str(msg) {
            Ok(j) => j,
            Err(_) => {
                let _ = writeln!(stream, "{}", r#"{"status":"error","reason":"invalid json"}"#);
                let _ = stream.flush();
                continue;
            }
        };
        let cmd = json.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        if cmd == "config" {
            // 配网：保存 bindToken，发 Connect，立即回 connecting；手机可离开热点，STA 连上后会在 WiFi 里持续发 binding，手机在原 WiFi 收听后回信完成绑定
            let bind_token = json.get("phone").and_then(|v| v.as_str()).map(String::from);
            if let Some(ref token) = bind_token {
                let mut guard = pending_bind_token.lock().unwrap();
                *guard = Some(token.clone());
            }
            let ssid = json
                .get("ssid")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let pwd = json.get("pwd").and_then(|s| s.as_str()).map(String::from);
            let sec = json.get("sec").and_then(|s| s.as_u64()).unwrap_or(3) as u8;
            let auth = match sec {
                0 => "open",
                1 => "wep",
                2 => "wpa",
                3 => "wpa2",
                4 => "wpa3",
                5 => "wpa2_enterprise",
                _ => "wpa2",
            };
            let (reply_tx, reply_rx) = mpsc::channel();
            thread::spawn(move || {
                let _ = reply_rx.recv();
            });
            if wifi_cmd_tx
                .send((
                    WifiCommand::Connect {
                        ssid,
                        password: pwd,
                        username: None,
                        auth: auth.to_string(),
                    },
                    reply_tx,
                ))
                .is_err()
            {
                let mut guard = pending_bind_token.lock().unwrap();
                *guard = None;
                let _ = writeln!(stream, "{}", r#"{"status":"error","reason":"internal"}"#);
                let _ = stream.flush();
                continue;
            }
            let _ = writeln!(stream, "{}", r#"{"status":"connecting"}"#);
            let _ = stream.flush();
            continue;
        }
        let (response, do_stop_ap_after_send) =
            process_ap_message(msg, &device_id, &fw_version, &wifi_cmd_tx, &on_config_success);
        if let Err(e) = writeln!(stream, "{}", response) {
            warn!("AP TCP write failed: {}", e);
            break;
        }
        if let Err(e) = stream.flush() {
            warn!("AP TCP flush failed: {}", e);
            break;
        }
        if do_stop_ap_after_send {
            let (stop_tx, stop_rx) = mpsc::channel();
            let _ = wifi_cmd_tx.send((WifiCommand::StopAp, stop_tx));
            let _ = stop_rx.recv_timeout(Duration::from_secs(5));
        }
    }
}

/// 返回 (回复内容, 是否在发送并 flush 后执行 StopAp)
fn process_ap_message(
    msg: &str,
    device_id: &str,
    fw_version: &str,
    wifi_cmd_tx: &WifiCmdTx,
    on_config_success: &Option<mpsc::Sender<(String, String)>>,
) -> (String, bool) {
    let json: serde_json::Value = match serde_json::from_str(msg) {
        Ok(j) => j,
        Err(_) => return (r#"{"status":"error","reason":"invalid json"}"#.to_string(), false),
    };
    let cmd = json.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    match cmd {
        "identify" => (
            format!(
                r#"{{"deviceId":"{}","fw":"{}"}}"#,
                escape_json(device_id),
                escape_json(fw_version)
            ),
            false,
        ),
        "config" => unreachable!("config handled in handle_ap_client"),
        _ => (r#"{"status":"error","reason":"unknown cmd"}"#.to_string(), false),
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}
