import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../models/wifi_network.dart';

/// Wi-Fi 凭据加密存储（与设计 2.2 一致）
class WifiStorageService {
  WifiStorageService() : _storage = const FlutterSecureStorage(aOptions: AndroidOptions(encryptedSharedPreferences: true));

  final FlutterSecureStorage _storage;
  static const _keyList = 'wifi_networks_list';
  static const _keyPrefix = 'wifi_';

  /// 存储的 SSID 列表（用于顺序与去重）
  Future<List<String>> _getSsidList() async {
    final raw = await _storage.read(key: _keyList);
    if (raw == null || raw.isEmpty) return [];
    try {
      final list = jsonDecode(raw) as List<dynamic>?;
      return list?.map((e) => e as String).toList() ?? [];
    } catch (_) {
      return [];
    }
  }

  Future<void> _setSsidList(List<String> list) async {
    await _storage.write(key: _keyList, value: jsonEncode(list));
  }

  Future<List<WifiNetwork>> getAll() async {
    final ssids = await _getSsidList();
    final out = <WifiNetwork>[];
    for (final ssid in ssids) {
      final raw = await _storage.read(key: _keyPrefix + ssid);
      if (raw == null) continue;
      try {
        final w = WifiNetwork.fromJson(jsonDecode(raw) as Map<String, dynamic>?);
        if (w != null) out.add(w);
      } catch (_) {}
    }
    return out;
  }

  Future<void> save(WifiNetwork network) async {
    final list = await _getSsidList();
    if (!list.contains(network.ssid)) {
      list.add(network.ssid);
      await _setSsidList(list);
    }
    await _storage.write(
      key: _keyPrefix + network.ssid,
      value: jsonEncode(network.toJson()),
    );
  }

  Future<void> delete(String ssid) async {
    final list = await _getSsidList();
    list.remove(ssid);
    await _setSsidList(list);
    await _storage.delete(key: _keyPrefix + ssid);
  }

  Future<WifiNetwork?> getBySsid(String ssid) async {
    final raw = await _storage.read(key: _keyPrefix + ssid);
    if (raw == null) return null;
    try {
      return WifiNetwork.fromJson(jsonDecode(raw) as Map<String, dynamic>?);
    } catch (_) {
      return null;
    }
  }
}
