/// Wi-Fi 网络信息（与设计文档一致）
class WifiNetwork {
  WifiNetwork({
    required this.ssid,
    required this.password,
    this.securityType = 2,
    this.identity,
    this.anonymousIdentity,
    this.caCertificate,
  });

  final String ssid;
  final String password;
  /// 0:OPEN, 1:WEP, 2:WPA, 3:WPA2, 4:WPA3, 5:Enterprise
  final int securityType;
  final String? identity;
  final String? anonymousIdentity;
  final String? caCertificate;

  Map<String, dynamic> toJson() => {
        'ssid': ssid,
        'password': password,
        'securityType': securityType,
        if (identity != null) 'identity': identity,
        if (anonymousIdentity != null) 'anonymousIdentity': anonymousIdentity,
        if (caCertificate != null) 'caCertificate': caCertificate,
      };

  static WifiNetwork? fromJson(Map<String, dynamic>? json) {
    if (json == null || json['ssid'] == null) return null;
    return WifiNetwork(
      ssid: json['ssid'] as String,
      password: json['password'] as String? ?? '',
      securityType: json['securityType'] as int? ?? 2,
      identity: json['identity'] as String?,
      anonymousIdentity: json['anonymousIdentity'] as String?,
      caCertificate: json['caCertificate'] as String?,
    );
  }
}
