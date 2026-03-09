import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_storage_service.dart';
import '../services/discovered_devices_store.dart';

class DeviceDetailScreen extends StatefulWidget {
  const DeviceDetailScreen({super.key, required this.device});

  final Device device;

  @override
  State<DeviceDetailScreen> createState() => _DeviceDetailScreenState();
}

class _DeviceDetailScreenState extends State<DeviceDetailScreen> {
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  late Device _device;
  final _nicknameController = TextEditingController();
  final _pairTargetIdController = TextEditingController();
  int _servo0Angle = 90;
  int _servo1Angle = 90;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _device = widget.device;
    _nicknameController.text = _device.name;
    _checkPendingPairRequests();
  }

  @override
  void dispose() {
    _nicknameController.dispose();
    _pairTargetIdController.dispose();
    super.dispose();
  }

  Future<void> _checkPendingPairRequests() async {
    if (_device.ipAddress.isEmpty) return;
    final res = await DeviceDiscoveryService.getPendingPairRequests(_device.ipAddress);
    if (!mounted) return;
    final pending = res['pending'] as List<dynamic>? ?? [];
    if (pending.isEmpty) return;
    final first = pending.first as Map<String, dynamic>?;
    final fromId = first?['from_device_id'] as String? ?? '';
    final fromIp = first?['from_ip'] as String? ?? '';
    if (fromId.isEmpty) return;
    final l10n = AppLocalizations.of(context);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.t('pair_request_title')),
        content: Text(l10n.t('pair_request_content', [fromId, fromIp])),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.t('reject'))),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: Text(l10n.t('accept'))),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    final acceptRes = await DeviceDiscoveryService.acceptPair(_device.ipAddress, fromId);
    if (!mounted) return;
    if (acceptRes['status'] == 'ok') {
      await _deviceStorage.save(_device.copyWith(pairedWithDeviceId: fromId));
      setState(() => _device = _device.copyWith(pairedWithDeviceId: fromId));
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.t('pairing_accepted'))),
        );
      }
    } else {
      _showError(acceptRes['reason']?.toString() ?? l10n.t('accept_pair_failed'));
    }
  }

  @override
  Widget build(BuildContext context) {
    final d = _device;
    final l10n = AppLocalizations.of(context);
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (bool didPop, dynamic result) {
        if (!didPop) Navigator.of(context).pop(_device);
      },
      child: Scaffold(
      appBar: AppBar(
        title: Text(d.name),
        actions: [
          IconButton(
            icon: const Icon(Icons.link_off),
            onPressed: () async {
              final navigator = Navigator.of(context);
              final ok = await showDialog<bool>(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: Text(l10n.t('unbind_title')),
                  content: Text(l10n.t('unbind_confirm')),
                  actions: [
                    TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.t('cancel'))),
                    FilledButton(onPressed: () => Navigator.pop(ctx, true), child: Text(l10n.t('unbind'))),
                  ],
                ),
              );
              if (ok == true) {
                await DeviceDiscoveryService.unbind(d.ipAddress);
                await _deviceStorage.delete(d.deviceId);
                if (!mounted) return;
                navigator.pop();
              }
            },
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          ListTile(
            title: Text(l10n.t('device_id')),
            subtitle: Text(d.deviceId),
            trailing: Icon(Icons.copy, size: 20, color: Theme.of(context).colorScheme.outline),
            onTap: () {
              Clipboard.setData(ClipboardData(text: d.deviceId));
              ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('copied_to_clipboard'))));
            },
          ),
          ListTile(
            title: Text(l10n.t('nickname')),
            subtitle: TextField(
              controller: _nicknameController,
              decoration: InputDecoration(
                hintText: l10n.t('nickname_hint'),
                isDense: true,
                border: const OutlineInputBorder(),
              ),
              onSubmitted: (v) => _saveNickname(v.trim().isEmpty ? d.deviceId : v.trim()),
            ),
            trailing: TextButton(
              onPressed: () => _saveNickname(_nicknameController.text.trim().isEmpty ? d.deviceId : _nicknameController.text.trim()),
              child: Text(l10n.t('save')),
            ),
          ),
          if (d.pairedWithDeviceId != null)
            ListTile(
              title: Text(l10n.t('paired')),
              subtitle: Text(d.pairedWithDeviceId!),
              trailing: Icon(Icons.copy, size: 20, color: Theme.of(context).colorScheme.outline),
              onTap: () {
                Clipboard.setData(ClipboardData(text: d.pairedWithDeviceId!));
                ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('copied_to_clipboard'))));
              },
            ),
          ListTile(
            title: const Text('IP'),
            subtitle: Text(d.ipAddress.isEmpty ? '—' : d.ipAddress),
          ),
          Divider(height: 24),
          Text(l10n.t('pair_section_title'), style: const TextStyle(fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          TextField(
            controller: _pairTargetIdController,
            decoration: InputDecoration(
              labelText: l10n.t('pair_target_label'),
              hintText: l10n.t('pair_target_hint'),
              border: const OutlineInputBorder(),
              isDense: true,
            ),
          ),
          const SizedBox(height: 8),
          FilledButton(
            onPressed: _busy ? null : _sendPairRequest,
            child: Text(l10n.t('send_pair_request')),
          ),
          const Divider(height: 24),
          Text(l10n.t('servo_0'), style: const TextStyle(fontWeight: FontWeight.bold)),
          Slider(
            value: _servo0Angle.toDouble(),
            min: 0,
            max: 180,
            divisions: 18,
            label: '$_servo0Angle°',
            onChanged: _busy ? null : (v) => setState(() => _servo0Angle = v.round()),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              FilledButton(
                onPressed: _busy ? null : () => _sendMoveServo(0, _servo0Angle),
                child: Text(l10n.t('set_angle')),
              ),
              const SizedBox(width: 16),
              OutlinedButton(
                onPressed: _busy ? null : _demoServo,
                child: Text(l10n.t('demo_servo')),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Text(l10n.t('servo_1'), style: const TextStyle(fontWeight: FontWeight.bold)),
          Slider(
            value: _servo1Angle.toDouble(),
            min: 0,
            max: 180,
            divisions: 18,
            label: '$_servo1Angle°',
            onChanged: _busy ? null : (v) => setState(() => _servo1Angle = v.round()),
          ),
          Center(
            child: FilledButton(
              onPressed: _busy ? null : () => _sendMoveServo(1, _servo1Angle),
              child: Text(l10n.t('set_angle')),
            ),
          ),
        ],
      ),
      ),
    );
  }

  void _showError(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.red.shade700),
    );
  }

  Future<void> _sendMoveServo(int index, int angle) async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null) return;
    if (_device.ipAddress.isEmpty) {
      _showError(l10n.t('device_ip_empty'));
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.moveServo(_device.ipAddress, index, angle);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(res['reason']?.toString() ?? l10n.t('set_failed'));
    }
  }

  Future<void> _demoServo() async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null) return;
    if (_device.ipAddress.isEmpty) {
      _showError(l10n.t('device_ip_empty'));
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.demoServo(_device.ipAddress);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(res['reason']?.toString() ?? l10n.t('demo_failed'));
    }
  }

  Future<void> _saveNickname(String name) async {
    final l10n = AppLocalizations.maybeOf(context);
    final next = _device.copyWith(name: name);
    await _deviceStorage.save(next);
    if (!mounted) return;
    setState(() => _device = next);
    if (l10n != null) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('nickname_saved'))));
    }
  }

  Future<void> _sendPairRequest() async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null) return;
    final targetId = _pairTargetIdController.text.trim();
    if (targetId.isEmpty) {
      _showError(l10n.t('enter_target_id'));
      return;
    }
    if (_device.ipAddress.isEmpty) {
      _showError(l10n.t('my_device_ip_empty'));
      return;
    }
    final targetIp = DiscoveredDevicesStore.getIp(targetId);
    if (targetIp == null) {
      _showError(l10n.t('device_not_found'));
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.pairRequest(_device.ipAddress, targetId, targetIp);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] == 'ok') {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('pair_request_sent'))));
    } else {
      _showError(res['reason']?.toString() ?? l10n.t('pair_request_failed'));
    }
  }
}
