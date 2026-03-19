/// 设备信息（与设计文档一致）
const Object _noChange = Object();

class Device {
  Device({
    required this.deviceId,
    required this.name,
    required this.ipAddress,
    this.isOnline = true,
    DateTime? lastSeen,
    this.isBound = true,
    this.boundPhoneId,
    this.pairedWithDeviceId,
    this.lastConnectedSsid,
    this.triggeredByPairCount = 0,
    this.isPeerShadow = false,
  }) : lastSeen = lastSeen ?? DateTime.now();

  final String deviceId;
  final String name;
  final String ipAddress;
  final bool isOnline;
  final DateTime lastSeen;
  final bool isBound;
  final String? boundPhoneId;
  final String? pairedWithDeviceId;

  /// 设备当前/最后连接的 WiFi 名称（由 UDP heartbeat/binding 上报）
  final String? lastConnectedSsid;

  /// 被配对设备远程触发的次数（由 ESP get_pair_status 的 triggered_count 同步）
  final int triggeredByPairCount;

  /// 是否为“对方设备镜像记录”（仅用于展示配对关系，不允许本机直接控制）。
  final bool isPeerShadow;

  Device copyWith({
    String? deviceId,
    String? name,
    String? ipAddress,
    bool? isOnline,
    DateTime? lastSeen,
    bool? isBound,
    Object? boundPhoneId = _noChange,
    Object? pairedWithDeviceId = _noChange,
    Object? lastConnectedSsid = _noChange,
    int? triggeredByPairCount,
    bool? isPeerShadow,
  }) {
    return Device(
      deviceId: deviceId ?? this.deviceId,
      name: name ?? this.name,
      ipAddress: ipAddress ?? this.ipAddress,
      isOnline: isOnline ?? this.isOnline,
      lastSeen: lastSeen ?? this.lastSeen,
      isBound: isBound ?? this.isBound,
      boundPhoneId: identical(boundPhoneId, _noChange)
          ? this.boundPhoneId
          : boundPhoneId as String?,
      pairedWithDeviceId: identical(pairedWithDeviceId, _noChange)
          ? this.pairedWithDeviceId
          : pairedWithDeviceId as String?,
      lastConnectedSsid: identical(lastConnectedSsid, _noChange)
          ? this.lastConnectedSsid
          : lastConnectedSsid as String?,
      triggeredByPairCount: triggeredByPairCount ?? this.triggeredByPairCount,
      isPeerShadow: isPeerShadow ?? this.isPeerShadow,
    );
  }

  Map<String, dynamic> toJson() => {
    'deviceId': deviceId,
    'name': name,
    'ipAddress': ipAddress,
    'isOnline': isOnline,
    'lastSeen': lastSeen.toIso8601String(),
    'isBound': isBound,
    if (boundPhoneId != null && boundPhoneId!.isNotEmpty)
      'boundPhoneId': boundPhoneId,
    if (pairedWithDeviceId != null) 'pairedWithDeviceId': pairedWithDeviceId,
    if (lastConnectedSsid != null && lastConnectedSsid!.isNotEmpty)
      'lastConnectedSsid': lastConnectedSsid,
    'triggeredByPairCount': triggeredByPairCount,
    'isPeerShadow': isPeerShadow,
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
      boundPhoneId: json['boundPhoneId'] as String?,
      pairedWithDeviceId: json['pairedWithDeviceId'] as String?,
      lastConnectedSsid: json['lastConnectedSsid'] as String?,
      triggeredByPairCount:
          (json['triggeredByPairCount'] as num?)?.toInt() ?? 0,
      isPeerShadow: json['isPeerShadow'] as bool? ?? false,
    );
  }
}
