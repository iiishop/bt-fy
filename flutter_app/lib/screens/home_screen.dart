import 'package:flutter/material.dart';

import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_storage_service.dart';
import '../services/pending_bind_store.dart';
import 'add_device_screen.dart';
import 'device_detail_screen.dart';
import 'wifi_manage_screen.dart';

/// 已绑定设备列表（设计 2.4）；回到原 AP 后通过 UDP 搜索 butterfly 设备并仅更新已配对设备的 IP/在线状态
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  late final DeviceDiscoveryService _discovery = DeviceDiscoveryService(
    onDeviceSeen: (device) {
      if (!mounted) return;
      setState(() {
        final i = _devices.indexWhere((d) => d.deviceId == device.deviceId);
        if (i >= 0) {
          final newIp = device.ipAddress.isEmpty || device.ipAddress == '0.0.0.0'
              ? _devices[i].ipAddress
              : device.ipAddress;
          _devices[i] = _devices[i].copyWith(
            ipAddress: newIp,
            lastSeen: device.lastSeen,
            isOnline: true,
          );
          _deviceStorage.save(_devices[i]);
        } else {
          _discoveredUnbound[device.deviceId] = device.copyWith(isBound: false);
        }
      });
    },
    onBindingSeen: (deviceId, ip, bindToken) async {
      if (ip.isEmpty || ip == '0.0.0.0') return;
      final p = PendingBindStore.getPending();
      if (p == null || p.token != bindToken) return;
      final res = await DeviceDiscoveryService.bind(ip, p.phoneId);
      if (res['status'] != 'ok') return;
      PendingBindStore.clear();
      await _deviceStorage.save(Device(
        deviceId: deviceId,
        name: deviceId,
        ipAddress: ip,
        isBound: true,
      ));
      if (!mounted) return;
      setState(() {});
      _loadDevices();
    },
  );
  List<Device> _devices = [];
  final Map<String, Device> _discoveredUnbound = {};
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadDevices();
    _discovery.startListening();
  }

  @override
  void dispose() {
    _discovery.stopListening();
    super.dispose();
  }

  Future<void> _loadDevices() async {
    setState(() => _loading = true);
    final list = await _deviceStorage.getAll();
    setState(() {
      _devices = list;
      _loading = false;
      _discoveredUnbound.removeWhere((id, _) => list.any((d) => d.deviceId == id));
    });
  }

  void _addDiscoveredDevice(Device d) {
    _deviceStorage.save(d.copyWith(isBound: true, name: d.deviceId));
    setState(() => _discoveredUnbound.remove(d.deviceId));
    _loadDevices();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('智能设备'),
        actions: [
          IconButton(
            icon: const Icon(Icons.wifi),
            onPressed: () async {
              await Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => const WifiManageScreen()),
              );
            },
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadDevices,
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : CustomScrollView(
              slivers: [
                if (_devices.isNotEmpty)
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                      child: Text(
                        '已绑定设备',
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
                      ),
                    ),
                  ),
                if (_devices.isNotEmpty)
                  SliverList(
                    delegate: SliverChildBuilderDelegate(
                      (context, index) {
                        final d = _devices[index];
                        return ListTile(
                          leading: Icon(
                            d.isOnline ? Icons.check_circle : Icons.offline_bolt,
                            color: d.isOnline ? Colors.green : Colors.grey,
                          ),
                          title: Text(d.name),
                          subtitle: Text(d.ipAddress.isEmpty ? d.deviceId : '${d.ipAddress} · ${d.deviceId}'),
                          trailing: const Icon(Icons.chevron_right),
                          onTap: () async {
                            await Navigator.push(
                              context,
                              MaterialPageRoute(
                                builder: (_) => DeviceDetailScreen(device: d),
                              ),
                            );
                            _loadDevices();
                          },
                        );
                      },
                      childCount: _devices.length,
                    ),
                  ),
                if (_discoveredUnbound.isNotEmpty) ...[
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
                      child: Text(
                        '发现的设备（未绑定）',
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
                      ),
                    ),
                  ),
                  SliverList(
                    delegate: SliverChildBuilderDelegate(
                      (context, index) {
                        final d = _discoveredUnbound.values.elementAt(index);
                        return ListTile(
                          leading: const Icon(Icons.router, color: Colors.grey),
                          title: Text(d.deviceId),
                          subtitle: Text(d.ipAddress.isEmpty ? '等待 IP' : d.ipAddress),
                          trailing: FilledButton(
                            onPressed: () => _addDiscoveredDevice(d),
                            child: const Text('添加'),
                          ),
                        );
                      },
                      childCount: _discoveredUnbound.length,
                    ),
                  ),
                ],
                if (_devices.isEmpty && _discoveredUnbound.isEmpty)
                  SliverFillRemaining(
                    hasScrollBody: false,
                    child: Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const Icon(Icons.devices_other, size: 64, color: Colors.grey),
                          const SizedBox(height: 16),
                          const Text('暂无已绑定设备'),
                          const SizedBox(height: 24),
                          FilledButton.icon(
                            onPressed: _goAddDevice,
                            icon: const Icon(Icons.add),
                            label: const Text('添加设备'),
                          ),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
      floatingActionButton: FloatingActionButton(
        onPressed: _goAddDevice,
        child: const Icon(Icons.add),
      ),
    );
  }

  Future<void> _goAddDevice() async {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const AddDeviceScreen()),
    );
    _loadDevices();
  }
}
