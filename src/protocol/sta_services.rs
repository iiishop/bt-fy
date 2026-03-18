//! STA 模式：UDP 广播（hello/heartbeat）+ TCP 控制服务（设计 5.2）+ 局域网配对

use log::{info, warn};
use std::io::{Read, Write, ErrorKind};
use std::net::{TcpStream, SocketAddrV4, TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hardware::{ContinuousServoService, ServoService};
use crate::system::config::{STA_TCP_PORT, STA_UDP_PORT, SERVO2_CMD_NEUTRAL};

/// 绑定状态：是否已绑定、绑定的手机 ID
pub type BindingState = Arc<Mutex<(bool, Option<String>)>>;

/// 配对状态：待处理列表、已配对设备 ID、对方 IP（用于 remote_trigger）
pub type PairState = Arc<Mutex<(Vec<(String, String)>, Option<String>, Option<String>)>>;

/// 被 pair 设备远程触发的防抖与计数：(last_trigger_time_ms, triggered_count)
pub type TriggerState = Arc<Mutex<(u64, u32)>>;

/// B 端是否正在接收 A 的 sync_tick（用于 sync_stop 后首次 sync_tick 做 servo1+2 与计数）
pub type SyncRunning = Arc<Mutex<bool>>;

/// Flutter 同步的 WiFi 列表：(ssid, password, auth)。每次与 Flutter 通讯后可更新，供后续重连等使用。
pub type WifiListStore = Arc<Mutex<Vec<(String, Option<String>, String)>>>;

/// 收到配网成功（STA 已连接）时，启动 UDP 广播与 TCP 控制（各占一线程）
pub fn spawn_sta_services_on_connect(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    sta_ssid: Option<String>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ContinuousServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
    trigger_state: TriggerState,
    sync_running: SyncRunning,
    wifi_list_store: WifiListStore,
) {
    let did = device_id.clone();
    let ip = sta_ip.clone();
    let binding_state_udp = Arc::clone(&binding_state);
    thread::Builder::new()
        .name("sta-udp".into())
        .spawn(move || run_udp_broadcast(did, ip, bind_token, sta_ssid, binding_state_udp))
        .expect("spawn sta-udp");
        thread::Builder::new()
            .name("sta-tcp".into())
            .spawn(move || run_tcp_control(device_id, sta_ip, servo, servo2, binding_state, pair_state, trigger_state, sync_running, wifi_list_store))
            .expect("spawn sta-tcp");
}

const BINDING_INTERVAL_SECS: u64 = 4;

fn run_udp_broadcast(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    sta_ssid: Option<String>,
    binding_state: BindingState,
) {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            warn!("STA UDP bind failed: {}", e);
            return;
        }
    };
    let _ = socket.set_broadcast(true);
    let dest: SocketAddrV4 = format!("255.255.255.255:{}", STA_UDP_PORT)
        .parse()
        .unwrap_or_else(|_| "255.255.255.255:12345".parse().unwrap());
    loop {
        let (bound, _) = *binding_state.lock().unwrap_or_else(|e| e.into_inner());
        if bound {
            let msg = serde_json::json!({
                "evt": "heartbeat",
                "id": device_id,
                "ssid": sta_ssid.as_deref().unwrap_or(""),
            });
            let _ = socket.send_to(msg.to_string().as_bytes(), dest);
        } else if let Some(ref token) = bind_token {
            let msg = serde_json::json!({
                "evt": "binding",
                "id": device_id,
                "ip": sta_ip,
                "ssid": sta_ssid.as_deref().unwrap_or(""),
                "bindToken": token,
            });
            let _ = socket.send_to(msg.to_string().as_bytes(), dest);
        }
        thread::sleep(Duration::from_secs(BINDING_INTERVAL_SECS));
    }
}

fn run_tcp_control(
    device_id: String,
    _sta_ip: String,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ContinuousServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
    trigger_state: TriggerState,
    sync_running: SyncRunning,
    wifi_list_store: WifiListStore,
) {
    let addr = format!("0.0.0.0:{}", STA_TCP_PORT);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            warn!("STA TCP {} bind failed: {}", addr, e);
            return;
        }
    };
    info!("STA TCP control on {}", addr);
    for stream in listener.incoming().filter_map(Result::ok) {
        let peer_addr = stream.peer_addr().ok();
        if let Some(ref peer) = peer_addr {
            info!("STA TCP client connected from {}", peer);
        }
        let servo = servo.clone();
        let servo2 = servo2.clone();
        let binding_state = Arc::clone(&binding_state);
        let pair_state = Arc::clone(&pair_state);
        let wifi_list_store = Arc::clone(&wifi_list_store);
        let device_id = device_id.clone();
        let trigger_state = Arc::clone(&trigger_state);
        let sync_running = Arc::clone(&sync_running);
        thread::spawn(move || {
            handle_sta_client(stream, &device_id, &servo, &servo2, &binding_state, &pair_state, &trigger_state, &sync_running, &wifi_list_store, peer_addr);
        });
    }
}

/// 从 stream 读一行（到 \n 或 EOF），不依赖 try_clone（ESP32 上常不可用）
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

fn handle_sta_client(
    mut stream: TcpStream,
    device_id: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ContinuousServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
    trigger_state: &TriggerState,
    sync_running: &SyncRunning,
    wifi_list_store: &WifiListStore,
    peer_addr: Option<std::net::SocketAddr>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let line = match read_line(&mut stream) {
        Ok(s) => s,
        Err(e) => {
            // 对方断开或 ESP 资源类错误(如 11) 时少刷 warn；128 = not connected
            let ok_disconnect = matches!(e.kind(), ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                | ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof)
                || e.raw_os_error() == Some(128);
            let resource_err = e.raw_os_error() == Some(11); // EAGAIN / 资源暂不可用
            if ok_disconnect {
                info!("STA TCP client disconnected");
            } else if resource_err {
                info!("STA TCP read: client left or busy ({})", e);
            } else {
                warn!("STA TCP read error: {}", e);
            }
            return;
        }
    };
    let msg = line.trim();
    if msg.is_empty() {
        return;
    }
    info!("STA TCP cmd: {}", if msg.len() > 80 { format!("{}...", &msg[..80]) } else { msg.to_string() });
    let response = process_sta_command(msg, device_id, servo, servo2, binding_state, pair_state, trigger_state, sync_running, wifi_list_store, peer_addr);
    if writeln!(stream, "{}", response).is_err() || stream.flush().is_err() {
        warn!("STA TCP write error");
    }
}

fn process_sta_command(
    msg: &str,
    device_id: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ContinuousServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
    trigger_state: &TriggerState,
    sync_running: &SyncRunning,
    wifi_list_store: &WifiListStore,
    peer_addr: Option<std::net::SocketAddr>,
) -> String {
    let json: serde_json::Value = match serde_json::from_str(msg) {
        Ok(j) => j,
        Err(_) => return r#"{"status":"error","reason":"invalid json"}"#.to_string(),
    };
    let cmd = json.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    match cmd {
        "demo_servo" => {
            match servo.as_ref().map(|s| s.demo_sequence()) {
                Some(Ok(())) => r#"{"status":"ok"}"#.to_string(),
                Some(Err(e)) => format!(r#"{{"status":"error","reason":"{}"}}"#, e),
                None => r#"{"status":"error","reason":"servo unavailable"}"#.to_string(),
            }
        }
        "move_servo" => {
            let idx = json.get("servo").and_then(|n| n.as_u64()).unwrap_or(0);
            let angle = json.get("angle").and_then(|n| n.as_u64()).unwrap_or(90) as u16;
            if idx == 0 {
                match servo.as_ref() {
                    Some(s) => match s.set_angle(angle.min(300)) {
                        Ok(()) => r#"{"status":"ok"}"#.to_string(),
                        Err(e) => format!(r#"{{"status":"error","reason":"{}"}}"#, e),
                    },
                    None => r#"{"status":"error","reason":"servo unavailable"}"#.to_string(),
                }
            } else {
                match servo2.as_ref() {
                    Some(s) => match s.set_angle(angle.min(180)) {
                        Ok(()) => r#"{"status":"ok"}"#.to_string(),
                        Err(e) => format!(r#"{{"status":"error","reason":"{}"}}"#, e),
                    },
                    None => r#"{"status":"error","reason":"servo unavailable"}"#.to_string(),
                }
            }
        }
        "bind" => {
            let phone = json
                .get("phone")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let mut state = binding_state.lock().unwrap_or_else(|e| e.into_inner());
            *state = (true, Some(phone));
            r#"{"status":"ok"}"#.to_string()
        }
        "unbind" => {
            let mut state = binding_state.lock().unwrap_or_else(|e| e.into_inner());
            *state = (false, None);
            r#"{"status":"ok"}"#.to_string()
        }
        "pair_request" => {
            if let (Some(target_ip), Some(target_device_id)) = (
                json.get("target_ip").and_then(|v| v.as_str()),
                json.get("target_device_id").and_then(|v| v.as_str()),
            ) {
                let addr = format!("{}:{}", target_ip, STA_TCP_PORT);
                if let Ok(mut other) = TcpStream::connect_timeout(
                    &addr.parse().unwrap_or_else(|_| "127.0.0.1:12345".parse().unwrap()),
                    Duration::from_secs(5),
                ) {
                    let req = serde_json::json!({"cmd":"pair_request","from_device_id": device_id});
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                    let mut line = Vec::new();
                    let mut buf = [0u8; 1];
                    let _ = other.set_read_timeout(Some(Duration::from_secs(3)));
                    while other.read(&mut buf).ok() == Some(1) {
                        if buf[0] == b'\n' {
                            break;
                        }
                        if buf[0] != b'\r' {
                            line.push(buf[0]);
                        }
                    }
                    let resp = String::from_utf8_lossy(&line);
                    if resp.contains("pair_accepted") || resp.contains("\"status\":\"ok\"") {
                        let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
                        state.1 = Some(target_device_id.to_string());
                        state.2 = Some(target_ip.to_string());
                    }
                    return r#"{"status":"ok"}"#.to_string();
                }
                return r#"{"status":"error","reason":"connect_target_failed"}"#.to_string();
            }
            if let Some(from_id) = json.get("from_device_id").and_then(|v| v.as_str()) {
                let from_ip = peer_addr
                    .and_then(|a| a.to_string().split(':').next().map(String::from))
                    .unwrap_or_else(|| "0.0.0.0".to_string());
                let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
                state.0.push((from_id.to_string(), from_ip));
                return r#"{"status":"ok","message":"pending"}"#.to_string();
            }
            r#"{"status":"error","reason":"missing target_ip or from_device_id"}"#.to_string()
        }
        "get_pending_pair_requests" => {
            let state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
            let pending: Vec<serde_json::Value> = state
                .0
                .iter()
                .map(|(id, ip)| serde_json::json!({"from_device_id": id, "from_ip": ip}))
                .collect();
            serde_json::json!({"status":"ok","pending": pending}).to_string()
        }
        "get_pair_status" => {
            let pair_guard = pair_state.lock().unwrap_or_else(|e| e.into_inner());
            let paired_with = pair_guard.1.as_deref().unwrap_or("");
            let trig = trigger_state.lock().unwrap_or_else(|e| e.into_inner());
            serde_json::json!({"status":"ok","paired_with": paired_with, "triggered_count": trig.1}).to_string()
        }
        "sync_tick" => {
            // A 触发时每 tick 发来 phase，B 的 servo1 与 A 基本同步。
            //
            // B 的 servo2（连续旋转舵机）规则：
            // - 当距离上一次 servo2“启动旋转”超过 10s 时（或初始时），执行一次：
            //   cmd=120 旋转 0.4s，然后回到 cmd=90（停止）
            // - 在 10s 窗口内无效（无论 A 是否短促/持续触发），超过 10s 则再次触发
            const ANGLE_MIN: u16 = 25;
            const ANGLE_MAX: u16 = 130;
            const SPAN: u16 = 105;
            const SERVO2_ROTATE_CMD: u16 = 120; // 暂定“速度值”
            const SERVO2_ROTATE_DURATION_MS: u64 = 400;
            const SERVO2_ROTATE_INTERVAL_MS: u64 = 10_000;
            let phase = json.get("phase").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let angle0 = if phase < 1.0 {
                ANGLE_MAX - (SPAN as f32 * phase) as u16
            } else {
                ANGLE_MIN + (SPAN as f32 * (phase - 1.0).min(1.0)) as u16
            };
            if let Some(s0) = servo.as_ref() {
                let _ = s0.set_angle(angle0);
            }
            // 保持同步状态标记：用于外部理解/调试
            {
                let mut run = sync_running.lock().unwrap_or_else(|e| e.into_inner());
                *run = true;
            }

            // servo2 rotation scheduler
            if let Some(s1) = servo2.as_ref() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let mut trig = trigger_state.lock().unwrap_or_else(|e| e.into_inner());
                let should_rotate = now_ms.saturating_sub(trig.0) >= SERVO2_ROTATE_INTERVAL_MS;
                if should_rotate {
                    trig.0 = now_ms;
                    trig.1 = trig.1.saturating_add(1);

                    // Spawn a short, non-blocking rotation task.
                    let s1_for_task = s1.clone();
                    std::thread::spawn(move || {
                        let _ = s1_for_task.set_angle(SERVO2_ROTATE_CMD);
                        std::thread::sleep(std::time::Duration::from_millis(
                            SERVO2_ROTATE_DURATION_MS,
                        ));
                        let _ = s1_for_task.set_angle(SERVO2_CMD_NEUTRAL);
                    });
                }
            }
            r#"{"status":"ok"}"#.to_string()
        }
        "sync_stop" => {
            if let Some(s0) = servo.as_ref() {
                // 停止时根据“离 25° 还是 130°更近”就近停靠
                let current = s0.get_angle();
                let d25 = (current as i32 - 25).abs() as u16;
                let d130 = (current as i32 - 130).abs() as u16;
                let idle_angle = if d25 <= d130 { 25 } else { 130 };
                let _ = s0.set_angle(idle_angle);
            }
            let mut run = sync_running.lock().unwrap_or_else(|e| e.into_inner());
            *run = false;
            r#"{"status":"ok"}"#.to_string()
        }
        "accept_pair" => {
            let from_device_id = json.get("from_device_id").and_then(|v| v.as_str()).unwrap_or("");
            let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
            let from_ip = state.0.iter().find(|(id, _)| id == from_device_id).map(|(_, ip)| ip.clone());
            if let Some(ip) = from_ip.clone() {
                state.0.retain(|(id, _)| id != from_device_id);
                state.1 = Some(from_device_id.to_string());
                state.2 = Some(ip.clone());
                drop(state);
                let addr = format!("{}:{}", ip, STA_TCP_PORT);
                if let Ok(mut other) = TcpStream::connect_timeout(
                    &addr.parse().unwrap_or_else(|_| "127.0.0.1:12345".parse().unwrap()),
                    Duration::from_secs(5),
                ) {
                    let req = serde_json::json!({"cmd":"pair_accepted","device_id": device_id});
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                }
                return r#"{"status":"ok"}"#.to_string();
            }
            r#"{"status":"error","reason":"pending_not_found"}"#.to_string()
        }
        "pair_accepted" => {
            if let Some(peer_id) = json.get("device_id").and_then(|v| v.as_str()) {
                let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
                state.1 = Some(peer_id.to_string());
                // peer_ip 在 initiator 的 pair_request 里已根据 target_ip 写入
                return r#"{"status":"ok"}"#.to_string();
            }
            r#"{"status":"error","reason":"missing device_id"}"#.to_string()
        }
        "unpair" => {
            let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
            state.1 = None;
            state.2 = None;
            state.0.clear();
            let peer_ip = json.get("peer_ip").and_then(|v| v.as_str());
            drop(state);
            if let Some(ip) = peer_ip {
                let addr = format!("{}:{}", ip, STA_TCP_PORT);
                if let Ok(mut other) = TcpStream::connect_timeout(
                    &addr.parse().unwrap_or_else(|_| "127.0.0.1:12345".parse().unwrap()),
                    Duration::from_secs(5),
                ) {
                    let req = serde_json::json!({"cmd":"unpair_notify","device_id": device_id});
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                }
            }
            r#"{"status":"ok"}"#.to_string()
        }
        "unpair_notify" => {
            if json.get("device_id").is_some() {
                let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
                state.1 = None;
                state.2 = None;
                return r#"{"status":"ok"}"#.to_string();
            }
            r#"{"status":"error","reason":"missing device_id"}"#.to_string()
        }
        "update_wifi_list" => {
            if let Some(networks) = json.get("networks").and_then(|n| n.as_array()) {
                let list: Vec<(String, Option<String>, String)> = networks
                    .iter()
                    .filter_map(|n| {
                        let obj = n.as_object()?;
                        let ssid = obj.get("ssid").and_then(|s| s.as_str())?.to_string();
                        let pwd = obj.get("pwd").and_then(|s| s.as_str()).map(String::from);
                        let sec = obj.get("sec").and_then(|s| s.as_u64()).unwrap_or(3) as u8;
                        let auth = match sec {
                            0 => "open",
                            1 => "wep",
                            2 => "wpa",
                            3 => "wpa2",
                            4 => "wpa3",
                            5 => "wpa2_enterprise",
                            _ => "wpa2",
                        };
                        Some((ssid, pwd, auth.to_string()))
                    })
                    .collect();
                let mut store = wifi_list_store.lock().unwrap_or_else(|e| e.into_inner());
                *store = list;
                return r#"{"status":"ok"}"#.to_string();
            }
            r#"{"status":"error","reason":"missing networks"}"#.to_string()
        }
        _ => r#"{"status":"error","reason":"unknown cmd"}"#.to_string(),
    }
}
