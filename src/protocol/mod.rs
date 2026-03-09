//! 与 App 的 JSON 协议（设计文档 5.1 / 5.2）

mod ap_tcp;
mod sta_services;

pub use ap_tcp::{start_ap_tcp_listener, PendingBindToken, PendingConfigDone};
pub use sta_services::{spawn_sta_services_on_connect, BindingState, PairState, WifiListStore};
