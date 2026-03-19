import 'package:android_id/android_id.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Provides a stable phone identifier for binding/auth flows.
///
/// Strategy:
/// 1) Use OS-level stable identifier (survives app reinstall in most cases).
/// 2) If unavailable, fail fast (no local UUID fallback).
class PhoneIdentityService {
  PhoneIdentityService()
    : _storage = const FlutterSecureStorage(
        aOptions: AndroidOptions(encryptedSharedPreferences: true),
      );

  final FlutterSecureStorage _storage;
  static const _storageKey = 'stable_phone_id_v2';
  static String? _cached;

  Future<String> getStablePhoneId() async {
    if (_cached != null && _cached!.isNotEmpty) return _cached!;

    final persisted = await _storage.read(key: _storageKey);
    if (persisted != null && persisted.isNotEmpty) {
      _cached = persisted;
      return persisted;
    }

    final osLevelId = await _tryGetOsStableId();
    if (osLevelId == null || osLevelId.isEmpty) {
      throw StateError('stable_phone_id_unavailable');
    }

    await _storage.write(key: _storageKey, value: osLevelId);
    _cached = osLevelId;
    return osLevelId;
  }

  Future<String?> _tryGetOsStableId() async {
    if (kIsWeb) return null;

    try {
      final androidId = await const AndroidId().getId();
      if (androidId != null && androidId.isNotEmpty) {
        return 'android_$androidId';
      }
    } catch (_) {}

    try {
      final info = await DeviceInfoPlugin().iosInfo;
      final idfv = info.identifierForVendor;
      if (idfv != null && idfv.isNotEmpty) {
        return 'ios_$idfv';
      }
    } catch (_) {}

    return null;
  }
}
