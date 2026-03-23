import 'dart:async';

import 'package:flutter/foundation.dart';

import '../constants/protocol.dart';
import '../models/device.dart';
import '../repositories/home_devices_repository.dart';
import '../services/device_discovery_service.dart';
import '../services/discovered_devices_store.dart';
import '../services/pending_bind_store.dart';

enum DevicePresence { online, suspected, offline }

class HomeViewModel extends ChangeNotifier {
  HomeViewModel({HomeDevicesRepository? repository})
    : _repository = repository ?? HomeDevicesRepository() {
    _discovery = DeviceDiscoveryService(
      onDeviceSeen: _onDeviceSeen,
      onBindingSeen: _onBindingSeen,
    );
  }

  final HomeDevicesRepository _repository;
  late final DeviceDiscoveryService _discovery;

  final ValueNotifier<int> statusTick = ValueNotifier<int>(0);

  Timer? _statusTimer;
  Timer? _pairStatusTimer;
  bool _pairPolling = false;

  List<Device> _devices = [];
  final Map<String, Device> _discoveredUnbound = {};
  bool _loading = true;

  List<Device> get devices => _devices;
  Map<String, Device> get discoveredUnbound => _discoveredUnbound;
  bool get loading => _loading;

  static const Duration suspectedTimeout = Duration(
    seconds: Protocol.heartbeatIntervalSeconds * 2,
  );
  static const Duration offlineTimeout = Duration(
    seconds: Protocol.heartbeatTimeoutSeconds,
  );

  static DevicePresence presenceOf(Device d, {DateTime? now}) {
    final current = now ?? DateTime.now();
    final diff = current.difference(d.lastSeen);
    final diffSeconds = diff.isNegative ? 0 : diff.inSeconds;
    if (diffSeconds < suspectedTimeout.inSeconds) {
      return DevicePresence.online;
    }
    if (diffSeconds < offlineTimeout.inSeconds) {
      return DevicePresence.suspected;
    }
    return DevicePresence.offline;
  }

  void start() {
    unawaited(loadDevices());
    _discovery.startListening();

    _statusTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      statusTick.value = statusTick.value + 1;
    });
    _pairStatusTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => unawaited(pollPairStatus()),
    );
  }

  Future<void> loadDevices() async {
    _loading = true;
    notifyListeners();
    final list = await _repository.getStoredDevices();
    _devices = list;
    _loading = false;
    _discoveredUnbound.removeWhere(
      (id, _) => list.any((d) => d.deviceId == id),
    );
    notifyListeners();
  }

  Future<void> addDiscoveredDevice(Device d, {required String phoneId}) async {
    await _repository.saveDevice(
      d.copyWith(isBound: true, name: d.deviceId, boundPhoneId: phoneId),
    );
    _discoveredUnbound.remove(d.deviceId);
    notifyListeners();
    await loadDevices();
  }

  Future<void> upsertDevice(Device d) async {
    final i = _devices.indexWhere((e) => e.deviceId == d.deviceId);
    if (i >= 0) {
      _devices[i] = d;
    } else {
      _devices = [..._devices, d];
    }
    await _repository.saveDevice(d);
    notifyListeners();
  }

  Future<void> pollPairStatus() async {
    if (_pairPolling || _devices.isEmpty) return;

    _pairPolling = true;
    try {
      final currentDevices = List<Device>.from(_devices);
      final updatedById = <String, Device>{
        for (final d in currentDevices) d.deviceId: d,
      };
      bool changed = false;

      for (final d in currentDevices) {
        final host = d.ipAddress;
        if (host.isEmpty) continue;

        final res = await _repository.getPairStatus(host);
        if (res['status'] != 'ok') continue;

        final pairedWith = (res['paired_with'] as String? ?? '').trim();
        final peerIpFromStatus = (res['peer_ip'] as String? ?? '').trim();
        final triggeredCount = (res['triggered_count'] as num?)?.toInt() ?? 0;

        if (pairedWith.isEmpty) {
          if (d.pairedWithDeviceId != null || d.triggeredByPairCount != 0) {
            final next = d.copyWith(
              pairedWithDeviceId: null,
              triggeredByPairCount: 0,
            );
            updatedById[d.deviceId] = next;
            changed = true;
            await _repository.saveDevice(next);
          }
          continue;
        }

        if (pairedWith == d.deviceId) continue; // ignore invalid self-pair

        if (d.pairedWithDeviceId != pairedWith ||
            d.triggeredByPairCount != triggeredCount) {
          final next = d.copyWith(
            pairedWithDeviceId: pairedWith,
            triggeredByPairCount: triggeredCount,
          );
          updatedById[d.deviceId] = next;
          changed = true;
          await _repository.saveDevice(next);
        }

        if (!updatedById.containsKey(pairedWith)) {
          final peerIp = peerIpFromStatus.isNotEmpty
              ? peerIpFromStatus
              : (DiscoveredDevicesStore.getIp(pairedWith) ?? '');
          if (peerIp.isNotEmpty) {
            final peer = Device(
              deviceId: pairedWith,
              name: pairedWith,
              ipAddress: peerIp,
              isBound: true,
              pairedWithDeviceId: d.deviceId,
              triggeredByPairCount: 0,
              isPeerShadow: true,
            );
            updatedById[pairedWith] = peer;
            changed = true;
            await _repository.saveDevice(peer);
          }
        }
      }

      if (changed) {
        _devices = updatedById.values.toList();
        _discoveredUnbound.removeWhere((id, _) => updatedById.containsKey(id));
        notifyListeners();
      }
    } finally {
      _pairPolling = false;
    }
  }

  void _onDeviceSeen(Device device) {
    final i = _devices.indexWhere((d) => d.deviceId == device.deviceId);
    if (i >= 0) {
      final stored = _devices[i];

      // ESP 在时序上可能会在短时间内发送 `hello`（isBound=false）：
      // 这不应该把“已绑定的设备”瞬间从 bound 列表里删掉，否则 UI 会在刷新时抖动
      // （你看到的：绑定后又变成 discovered / 成对后又变成只有 bound）。
      //
      // 所以：如果本地已是已绑定（含 peer shadow），则在收到 hello 时只更新在线信息，
      // 不切换为 isBound=false、也不触发 delete。
      if (!device.isBound && stored.isBound) {
        final newIp = device.ipAddress.isEmpty || device.ipAddress == '0.0.0.0'
            ? stored.ipAddress
            : device.ipAddress;
        _devices[i] = stored.copyWith(
          ipAddress: newIp,
          lastSeen: device.lastSeen,
          isOnline: true,
          // 保持 bound 状态不变，避免 UI 分类抖动
          isBound: true,
          lastConnectedSsid:
              device.lastConnectedSsid ?? stored.lastConnectedSsid,
        );
        unawaited(_repository.saveDevice(_devices[i]));
        _discoveredUnbound.remove(device.deviceId);
      } else {
        final newIp = device.ipAddress.isEmpty || device.ipAddress == '0.0.0.0'
            ? stored.ipAddress
            : device.ipAddress;
        _devices[i] = stored.copyWith(
          ipAddress: newIp,
          lastSeen: device.lastSeen,
          isOnline: true,
          isBound: device.isBound,
          lastConnectedSsid:
              device.lastConnectedSsid ?? stored.lastConnectedSsid,
        );
        unawaited(_repository.saveDevice(_devices[i]));
        _discoveredUnbound.remove(device.deviceId);
        if (!device.isBound) {
          // 极少数：本地记录本来就不是 bound，才允许按旧逻辑降级到 discovered。
          unawaited(_repository.deleteDevice(device.deviceId));
          _discoveredUnbound[device.deviceId] = device.copyWith(isBound: false);
          _devices.removeAt(i);
        }
      }
    } else if (!device.isBound) {
      _discoveredUnbound[device.deviceId] = device.copyWith(isBound: false);
    }

    DiscoveredDevicesStore.update(device.deviceId, device.ipAddress);
    notifyListeners();
  }

  Future<void> _onBindingSeen(
    String deviceId,
    String ip,
    String bindToken,
  ) async {
    if (ip.isEmpty || ip == '0.0.0.0') return;
    final p = await PendingBindStore.getPending();
    if (p == null || p.token != bindToken) return;
    final res = await _repository.bind(ip, p.phoneId);
    if (res['status'] != 'ok') return;
    await PendingBindStore.clear();

    await _repository.saveDevice(
      Device(
        deviceId: deviceId,
        name: deviceId,
        ipAddress: ip,
        isBound: true,
        boundPhoneId: p.phoneId,
      ),
    );
    final index = _devices.indexWhere((d) => d.deviceId == deviceId);
    if (index >= 0) {
      _devices[index] = _devices[index].copyWith(
        boundPhoneId: p.phoneId,
        ipAddress: ip,
      );
    }
    DiscoveredDevicesStore.update(deviceId, ip);
    _discoveredUnbound.remove(deviceId);
    await loadDevices();
  }

  @override
  void dispose() {
    _statusTimer?.cancel();
    _statusTimer = null;
    _pairStatusTimer?.cancel();
    _pairStatusTimer = null;
    _discovery.stopListening();
    statusTick.dispose();
    super.dispose();
  }
}
