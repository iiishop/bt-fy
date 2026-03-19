import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// 配网时暂存 (bindToken, phoneId)。
/// 使用安全存储持久化，避免切网/进程回收后丢失。
class PendingBindStore {
  PendingBindStore._();

  static const FlutterSecureStorage _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );
  static const String _storageKey = 'pending_bind_v1';

  static String? _token;
  static String? _phoneId;
  static bool _loaded = false;

  static Future<void> _ensureLoaded() async {
    if (_loaded) return;
    _loaded = true;
    final raw = await _storage.read(key: _storageKey);
    if (raw == null || raw.isEmpty) return;
    try {
      final map = jsonDecode(raw) as Map<String, dynamic>;
      final token = map['token'] as String?;
      final phoneId = map['phoneId'] as String?;
      if (token != null &&
          token.isNotEmpty &&
          phoneId != null &&
          phoneId.isNotEmpty) {
        _token = token;
        _phoneId = phoneId;
      }
    } catch (_) {}
  }

  static Future<void> setPending(String token, String phoneId) async {
    _token = token;
    _phoneId = phoneId;
    await _storage.write(
      key: _storageKey,
      value: jsonEncode({'token': token, 'phoneId': phoneId}),
    );
  }

  static Future<({String token, String phoneId})?> getPending() async {
    await _ensureLoaded();
    if (_token == null || _phoneId == null) return null;
    return (token: _token!, phoneId: _phoneId!);
  }

  static Future<void> clear() async {
    _token = null;
    _phoneId = null;
    await _storage.delete(key: _storageKey);
  }
}
