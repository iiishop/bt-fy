import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../models/device.dart';

/// 已绑定设备列表持久化（本地存储）
class DeviceStorageService {
  DeviceStorageService() : _storage = const FlutterSecureStorage(aOptions: AndroidOptions(encryptedSharedPreferences: true));

  final FlutterSecureStorage _storage;
  static const _keyList = 'bound_devices_list';
  static const _keyPrefix = 'device_';

  Future<List<String>> _getDeviceIds() async {
    final raw = await _storage.read(key: _keyList);
    if (raw == null || raw.isEmpty) return [];
    try {
      final list = jsonDecode(raw) as List<dynamic>?;
      return list?.map((e) => e as String).toList() ?? [];
    } catch (_) {
      return [];
    }
  }

  Future<void> _setDeviceIds(List<String> list) async {
    await _storage.write(key: _keyList, value: jsonEncode(list));
  }

  Future<List<Device>> getAll() async {
    final ids = await _getDeviceIds();
    final out = <Device>[];
    for (final id in ids) {
      final raw = await _storage.read(key: _keyPrefix + id);
      if (raw == null) continue;
      try {
        final d = Device.fromJson(jsonDecode(raw) as Map<String, dynamic>?);
        if (d != null) out.add(d);
      } catch (_) {}
    }
    return out;
  }

  Future<void> save(Device device) async {
    final list = await _getDeviceIds();
    if (!list.contains(device.deviceId)) {
      list.add(device.deviceId);
      await _setDeviceIds(list);
    }
    await _storage.write(
      key: _keyPrefix + device.deviceId,
      value: jsonEncode(device.toJson()),
    );
  }

  Future<void> delete(String deviceId) async {
    final list = await _getDeviceIds();
    list.remove(deviceId);
    await _setDeviceIds(list);
    await _storage.delete(key: _keyPrefix + deviceId);
  }

  Future<Device?> getByDeviceId(String deviceId) async {
    final raw = await _storage.read(key: _keyPrefix + deviceId);
    if (raw == null) return null;
    try {
      return Device.fromJson(jsonDecode(raw) as Map<String, dynamic>?);
    } catch (_) {
      return null;
    }
  }
}
