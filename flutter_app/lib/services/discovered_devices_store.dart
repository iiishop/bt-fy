/// 局域网内已发现的设备 ID -> 最新 IP（由 UDP 广播更新，用于配对时解析对方设备 ID）
class DiscoveredDevicesStore {
  DiscoveredDevicesStore._();

  static final Map<String, String> _deviceIdToIp = {};

  static void update(String deviceId, String ip) {
    if (ip.isEmpty || ip == '0.0.0.0') return;
    _deviceIdToIp[deviceId] = ip;
  }

  static String? getIp(String deviceId) => _deviceIdToIp[deviceId];

  static bool has(String deviceId) => _deviceIdToIp.containsKey(deviceId);
}
