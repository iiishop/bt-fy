/// 设备信息（与设计文档一致）
class Device {
  Device({
    required this.deviceId,
    required this.name,
    required this.ipAddress,
    this.isOnline = true,
    DateTime? lastSeen,
    this.isBound = true,
  }) : lastSeen = lastSeen ?? DateTime.now();

  final String deviceId;
  final String name;
  final String ipAddress;
  final bool isOnline;
  final DateTime lastSeen;
  /// 是否已绑定（待绑定列表中的设备为 false）
  final bool isBound;

  Device copyWith({
    String? deviceId,
    String? name,
    String? ipAddress,
    bool? isOnline,
    DateTime? lastSeen,
    bool? isBound,
  }) {
    return Device(
      deviceId: deviceId ?? this.deviceId,
      name: name ?? this.name,
      ipAddress: ipAddress ?? this.ipAddress,
      isOnline: isOnline ?? this.isOnline,
      lastSeen: lastSeen ?? this.lastSeen,
      isBound: isBound ?? this.isBound,
    );
  }

  Map<String, dynamic> toJson() => {
        'deviceId': deviceId,
        'name': name,
        'ipAddress': ipAddress,
        'isOnline': isOnline,
        'lastSeen': lastSeen.toIso8601String(),
        'isBound': isBound,
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
    );
  }
}
