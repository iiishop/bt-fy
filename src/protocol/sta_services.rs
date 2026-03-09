//! STA 模式：UDP 广播（hello/heartbeat）+ TCP 控制服务（设计 5.2）+ 局域网配对

use log::{info, warn};
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddrV4, TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hardware::ServoService;
use crate::system::config::{STA_TCP_PORT, STA_UDP_PORT};

/// 绑定状态：是否已绑定、绑定的手机 ID
pub type BindingState = Arc<Mutex<(bool, Option<String>)>>;

/// 配对状态：待处理列表 (from_device_id, from_ip)、已配对设备 ID
pub type PairState = Arc<Mutex<(Vec<(String, String)>, Option<String>)>>;

/// 收到配网成功（STA 已连接）时，启动 UDP 广播与 TCP 控制（各占一线程）
pub fn spawn_sta_services_on_connect(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
) {
    let did = device_id.clone();
    let ip = sta_ip.clone();
    let binding_state_udp = Arc::clone(&binding_state);
    thread::Builder::new()
        .name("sta-udp".into())
        .spawn(move || run_udp_broadcast(did, ip, bind_token, binding_state_udp))
        .expect("spawn sta-udp");
    thread::Builder::new()
        .name("sta-tcp".into())
        .spawn(move || run_tcp_control(device_id, sta_ip, servo, servo2, binding_state, pair_state))
        .expect("spawn sta-tcp");
}

const BINDING_INTERVAL_SECS: u64 = 4;

fn run_udp_broadcast(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
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
            let msg = format!(r#"{{"evt":"heartbeat","id":"{}"}}"#, device_id);
            let _ = socket.send_to(msg.as_bytes(), dest);
        } else if let Some(ref token) = bind_token {
            let msg = serde_json::json!({
                "evt": "binding",
                "id": device_id,
                "ip": sta_ip,
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
    servo2: Option<Arc<ServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
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
        let device_id = device_id.clone();
        thread::spawn(move || {
            handle_sta_client(stream, &device_id, &servo, &servo2, &binding_state, &pair_state, peer_addr);
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
    servo2: &Option<Arc<ServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
    peer_addr: Option<std::net::SocketAddr>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let line = match read_line(&mut stream) {
        Ok(s) => s,
        Err(e) => {
            warn!("STA TCP read error: {}", e);
            return;
        }
    };
    let msg = line.trim();
    if msg.is_empty() {
        return;
    }
    info!("STA TCP cmd: {}", if msg.len() > 80 { format!("{}...", &msg[..80]) } else { msg.to_string() });
    let response = process_sta_command(msg, device_id, servo, servo2, binding_state, pair_state, peer_addr);
    if writeln!(stream, "{}", response).is_err() || stream.flush().is_err() {
        warn!("STA TCP write error");
    }
}

fn process_sta_command(
    msg: &str,
    device_id: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
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
            let s = if idx == 0 { servo.as_ref() } else { servo2.as_ref() };
            match s {
                Some(s) => match s.set_angle(angle.min(300)) {
                    Ok(()) => r#"{"status":"ok"}"#.to_string(),
                    Err(e) => format!(r#"{{"status":"error","reason":"{}"}}"#, e),
                },
                None => r#"{"status":"error","reason":"servo unavailable"}"#.to_string(),
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
            if let (Some(target_ip), Some(_target_device_id)) = (
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
        "accept_pair" => {
            let from_device_id = json.get("from_device_id").and_then(|v| v.as_str()).unwrap_or("");
            let mut state = pair_state.lock().unwrap_or_else(|e| e.into_inner());
            let from_ip = state.0.iter().find(|(id, _)| id == from_device_id).map(|(_, ip)| ip.clone());
            if let Some(ip) = from_ip {
                state.0.retain(|(id, _)| id != from_device_id);
                state.1 = Some(from_device_id.to_string());
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
                return r#"{"status":"ok"}"#.to_string();
            }
            r#"{"status":"error","reason":"missing device_id"}"#.to_string()
        }
        _ => r#"{"status":"error","reason":"unknown cmd"}"#.to_string(),
    }
}
