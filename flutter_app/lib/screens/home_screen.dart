import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_storage_service.dart';
import '../services/discovered_devices_store.dart';
import '../services/pending_bind_store.dart';
import 'add_device_screen.dart';
import 'device_detail_screen.dart';
import 'wifi_manage_screen.dart';

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
        DiscoveredDevicesStore.update(device.deviceId, device.ipAddress);
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
      DiscoveredDevicesStore.update(deviceId, ip);
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
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.t('app_title')),
        actions: [
          TextButton(
            onPressed: () => l10n.setLocale(l10n.locale == 'en' ? 'zh' : 'en'),
            child: Text(l10n.languageToggleLabel, style: const TextStyle(fontSize: 14)),
          ),
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
                        l10n.t('bound_devices'),
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
                            final updated = await Navigator.push<Device>(
                              context,
                              MaterialPageRoute(
                                builder: (_) => DeviceDetailScreen(device: d),
                              ),
                            );
                            if (updated != null) {
                              final i = _devices.indexWhere((e) => e.deviceId == updated.deviceId);
                              if (i >= 0) {
                                setState(() => _devices[i] = updated);
                                await _deviceStorage.save(updated);
                              }
                            }
                            await _loadDevices();
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
                        l10n.t('discovered_devices'),
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
                          subtitle: Text(d.ipAddress.isEmpty ? l10n.t('waiting_ip') : d.ipAddress),
                          trailing: FilledButton(
                            onPressed: () => _addDiscoveredDevice(d),
                            child: Text(l10n.t('add')),
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
                          Text(l10n.t('no_bound_devices')),
                          const SizedBox(height: 24),
                          FilledButton.icon(
                            onPressed: _goAddDevice,
                            icon: const Icon(Icons.add),
                            label: Text(l10n.t('add_device')),
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
