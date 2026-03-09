/// 设备信息（与设计文档一致）
class Device {
  Device({
    required this.deviceId,
    required this.name,
    required this.ipAddress,
    this.isOnline = true,
    DateTime? lastSeen,
    this.isBound = true,
    this.pairedWithDeviceId,
    this.lastConnectedSsid,
    this.triggeredByPairCount = 0,
  }) : lastSeen = lastSeen ?? DateTime.now();

  final String deviceId;
  final String name;
  final String ipAddress;
  final bool isOnline;
  final DateTime lastSeen;
  final bool isBound;
  final String? pairedWithDeviceId;
  /// 设备当前/最后连接的 WiFi 名称（由 UDP heartbeat/binding 上报）
  final String? lastConnectedSsid;
  /// 被配对设备远程触发的次数（由 ESP get_pair_status 的 triggered_count 同步）
  final int triggeredByPairCount;

  Device copyWith({
    String? deviceId,
    String? name,
    String? ipAddress,
    bool? isOnline,
    DateTime? lastSeen,
    bool? isBound,
    String? pairedWithDeviceId,
    String? lastConnectedSsid,
    int? triggeredByPairCount,
  }) {
    return Device(
      deviceId: deviceId ?? this.deviceId,
      name: name ?? this.name,
      ipAddress: ipAddress ?? this.ipAddress,
      isOnline: isOnline ?? this.isOnline,
      lastSeen: lastSeen ?? this.lastSeen,
      isBound: isBound ?? this.isBound,
      pairedWithDeviceId: pairedWithDeviceId ?? this.pairedWithDeviceId,
      lastConnectedSsid: lastConnectedSsid ?? this.lastConnectedSsid,
      triggeredByPairCount: triggeredByPairCount ?? this.triggeredByPairCount,
    );
  }

  Map<String, dynamic> toJson() => {
        'deviceId': deviceId,
        'name': name,
        'ipAddress': ipAddress,
        'isOnline': isOnline,
        'lastSeen': lastSeen.toIso8601String(),
        'isBound': isBound,
        if (pairedWithDeviceId != null) 'pairedWithDeviceId': pairedWithDeviceId,
        if (lastConnectedSsid != null && lastConnectedSsid!.isNotEmpty) 'lastConnectedSsid': lastConnectedSsid,
        'triggeredByPairCount': triggeredByPairCount,
      };

  static Device? fromJson(Map<String, dynamic>? json) {
    if (json == null || json['deviceId'] == null) return null;
    return Device(
      deviceId: json['deviceId'] as String,
      name: json['name'] as String? ?? json['deviceId'] as String,
      ipAddress: json['ipAddress'] as String? ?? '',
      isOnline: json['isOnline'] as bool? ?? true,
      lastSeen: json['lastSeen'] != null
          ? DateTime.tryParse(json['lastSeen'] as String) ?? DateTime.now()
          : DateTime.now(),
      isBound: json['isBound'] as bool? ?? true,
      pairedWithDeviceId: json['pairedWithDeviceId'] as String?,
      lastConnectedSsid: json['lastConnectedSsid'] as String?,
      triggeredByPairCount: (json['triggeredByPairCount'] as num?)?.toInt() ?? 0,
    );
  }
}
