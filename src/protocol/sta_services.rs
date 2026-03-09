//! STA 模式：UDP 广播（hello/heartbeat）+ TCP 控制服务（设计 5.2）

use log::{info, warn};
use std::io::Read;
use std::net::{SocketAddrV4, TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hardware::ServoService;
use crate::system::config::{STA_TCP_PORT, STA_UDP_PORT};

/// 绑定状态：是否已绑定、绑定的手机 ID
pub type BindingState = Arc<Mutex<(bool, Option<String>)>>;

/// 收到配网成功（STA 已连接）时，启动 UDP 广播与 TCP 控制（各占一线程）
/// bind_token: 配网时手机传来的 token，有则持续发 evt=binding 直至收到 bind 回信；无则发 heartbeat
pub fn spawn_sta_services_on_connect(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ServoService>>,
    binding_state: BindingState,
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
        .spawn(move || run_tcp_control(device_id, sta_ip, servo, servo2, binding_state))
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
    _device_id: String,
    _sta_ip: String,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ServoService>>,
    binding_state: BindingState,
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
        if let Ok(peer) = stream.peer_addr() {
            info!("STA TCP client connected from {}", peer);
        }
        let servo = servo.clone();
        let servo2 = servo2.clone();
        let binding_state = Arc::clone(&binding_state);
        thread::spawn(move || {
            handle_sta_client(stream, &servo, &servo2, &binding_state);
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
    mut stream: std::net::TcpStream,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ServoService>>,
    binding_state: &BindingState,
) {
    use std::io::Write;
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
    let response = process_sta_command(msg, servo, servo2, binding_state);
    if writeln!(stream, "{}", response).is_err() || stream.flush().is_err() {
        warn!("STA TCP write error");
    }
}

fn process_sta_command(
    msg: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ServoService>>,
    binding_state: &BindingState,
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
        _ => r#"{"status":"error","reason":"unknown cmd"}"#.to_string(),
    }
}
