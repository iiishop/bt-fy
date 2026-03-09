/// 与设备端约定的协议常量（与设计文档 5.1 / 5.2 一致）
class Protocol {
  Protocol._();

  /// 设备热点下设备 IP（当前固件为 192.168.71.1）
  static const String deviceApGateway = '192.168.71.1';

  /// AP 模式配网 TCP 端口
  static const int apTcpPort = 1234;

  /// STA 模式：UDP 广播端口、TCP 控制端口
  static const int staUdpPort = 12345;
  static const int staTcpPort = 12345;

  /// 心跳超时视为离线（秒）
  static const int heartbeatTimeoutSeconds = 90;

  /// 设备广播心跳间隔约 30 秒
  static const int heartbeatIntervalSeconds = 30;
}
