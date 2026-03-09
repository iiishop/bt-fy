/// 配网时暂存 (bindToken, phoneId)，手机切回原 WiFi 后收到 ESP 的 binding 且 token 匹配时，发 bind 并加入列表后清除
class PendingBindStore {
  PendingBindStore._();

  static String? _token;
  static String? _phoneId;

  static void setPending(String token, String phoneId) {
    _token = token;
    _phoneId = phoneId;
  }

  static ({String token, String phoneId})? getPending() {
    if (_token == null || _phoneId == null) return null;
    return (token: _token!, phoneId: _phoneId!);
  }

  static void clear() {
    _token = null;
    _phoneId = null;
  }
}
