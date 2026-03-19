import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_provisioning_service.dart';
import '../services/device_storage_service.dart';
import '../services/discovered_devices_store.dart';
import '../services/phone_identity_service.dart';
import '../services/wifi_storage_service.dart';
import '../viewmodels/home_view_model.dart';

class DeviceDetailScreen extends StatefulWidget {
  const DeviceDetailScreen({super.key, required this.device});

  final Device device;

  @override
  State<DeviceDetailScreen> createState() => _DeviceDetailScreenState();
}

class _DeviceDetailScreenState extends State<DeviceDetailScreen> {
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  final WifiStorageService _wifiStorage = WifiStorageService();
  final PhoneIdentityService _phoneIdentity = PhoneIdentityService();
  late Device _device;
  final _nicknameController = TextEditingController();
  final _pairTargetIdController = TextEditingController();
  int _servo0Angle = 90;
  int _servo1Angle = 90;
  bool _busy = false;
  Timer? _statusTimer;
  bool get _isPeerShadow => _device.isPeerShadow;

  @override
  void initState() {
    super.initState();
    _device = widget.device;
    _nicknameController.text = _device.name;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      unawaited(_enforceOwnershipOnOpen());
    });
    if (!_isPeerShadow) {
      _syncWifiListThenCheckPending();
    }
    _statusTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() {});
    });
  }

  Future<void> _enforceOwnershipOnOpen() async {
    if (_isPeerShadow || !mounted) return;
    final bound = _device.boundPhoneId?.trim();
    if (bound == null || bound.isEmpty) {
      final l10n = AppLocalizations.of(context);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.t('device_owner_unknown'))));
      Navigator.of(context).pop();
      return;
    }
    String currentPhoneId;
    try {
      currentPhoneId = await _phoneIdentity.getStablePhoneId();
    } catch (_) {
      if (!mounted) return;
      final l10n = AppLocalizations.of(context);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.t('stable_phone_id_required'))),
      );
      Navigator.of(context).pop();
      return;
    }
    if (!mounted) return;
    if (currentPhoneId != bound) {
      final l10n = AppLocalizations.of(context);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.t('device_owner_mismatch'))),
      );
      Navigator.of(context).pop();
    }
  }

  /// 与设备通讯时先同步 WiFi 表、从 ESP 同步配对状态，再拉取配对请求
  Future<void> _syncWifiListThenCheckPending() async {
    if (_device.ipAddress.isEmpty) {
      return;
    }
    final list = await _wifiStorage.getAll();
    if (list.isNotEmpty) {
      final networks = list.map((w) => {'ssid': w.ssid, 'pwd': w.password, 'sec': w.securityType}).toList();
      await DeviceDiscoveryService.updateWifiList(_device.ipAddress, networks);
    }
    if (!mounted) return;
    final status = await DeviceDiscoveryService.getPairStatus(_device.ipAddress);
    if (!mounted) return;
    final pairedWith = (status['paired_with'] as String? ?? '').trim();
    final peerIpFromStatus = (status['peer_ip'] as String? ?? '').trim();
    final triggeredCount = (status['triggered_count'] as num?)?.toInt() ?? _device.triggeredByPairCount;
    if (pairedWith.isNotEmpty) {
      if (_device.pairedWithDeviceId != pairedWith || _device.triggeredByPairCount != triggeredCount) {
        final next = _device.copyWith(pairedWithDeviceId: pairedWith, triggeredByPairCount: triggeredCount);
        await _deviceStorage.save(next);
        setState(() => _device = next);
      }
      final peer = await _deviceStorage.getByDeviceId(pairedWith);
      if (peer == null) {
        final peerIp = peerIpFromStatus.isNotEmpty ? peerIpFromStatus : (DiscoveredDevicesStore.getIp(pairedWith) ?? '');
        await _deviceStorage.save(Device(
          deviceId: pairedWith,
          name: pairedWith,
          ipAddress: peerIp,
          isBound: true,
          pairedWithDeviceId: _device.deviceId,
          isPeerShadow: true,
        ));
      }
    } else {
      if (_device.pairedWithDeviceId != null) {
        final oldPeer = _device.pairedWithDeviceId!;
        await _deviceStorage.delete(oldPeer);
        await _deviceStorage.save(_device.copyWith(pairedWithDeviceId: null, triggeredByPairCount: 0));
        setState(() => _device = _device.copyWith(pairedWithDeviceId: null, triggeredByPairCount: 0));
      }
    }
    if (!mounted) return;
  }

  @override
  void dispose() {
    _nicknameController.dispose();
    _pairTargetIdController.dispose();
    _statusTimer?.cancel();
    _statusTimer = null;
    super.dispose();
  }

  String _statusText(AppLocalizations l10n, Device d) {
    switch (HomeViewModel.presenceOf(d)) {
      case DevicePresence.online:
        return l10n.t('currently_online');
      case DevicePresence.suspected:
        return l10n.t('suspected_offline', [_formatLastSeen(d.lastSeen)]);
      case DevicePresence.offline:
        return l10n.t('last_seen', [_formatLastSeen(d.lastSeen)]);
    }
  }

  Future<bool> _unbindWithProgress() async {
    final l10n = AppLocalizations.of(context);
    final currentContext = context;
    if (_device.ipAddress.isEmpty) return false;

    setState(() => _busy = true);

    final progressValue = ValueNotifier<double>(0.0);
    final progressText = ValueNotifier<String>(l10n.t('unbind_progress_sending'));

    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) {
        return AlertDialog(
          title: Text(l10n.t('unbind_progress_title')),
          content: SizedBox(
            width: 320,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                ValueListenableBuilder<String>(
                  valueListenable: progressText,
                  builder: (context, value, _) => Text(value),
                ),
                const SizedBox(height: 16),
                ValueListenableBuilder<double>(
                  valueListenable: progressValue,
                  builder: (context, value, _) => LinearProgressIndicator(value: value),
                ),
              ],
            ),
          ),
        );
      },
    );

    var dialogPopped = false;
    try {
      progressValue.value = 0.2;
      progressText.value = l10n.t('unbind_progress_sending');
      final res = await DeviceDiscoveryService.unbind(_device.ipAddress);
      if (res['status'] != 'ok') {
        throw Exception(
          DeviceProvisioningService.pickErrorMessage(res) ??
              l10n.t('unbind_progress_failed', ['unknown']),
        );
      }

      progressValue.value = 0.7;
      progressText.value = l10n.t('unbind_progress_clearing');

      await _deviceStorage.delete(_device.deviceId);

      progressValue.value = 1.0;
      progressText.value = l10n.t('unbind_progress_done');

      dialogPopped = true;
      if (!currentContext.mounted) return false;

      Navigator.of(currentContext, rootNavigator: true).pop();

      ScaffoldMessenger.of(currentContext).showSnackBar(
        SnackBar(content: Text(l10n.t('unbind_progress_done'))),
      );
      return true;
    } catch (e) {
      if (currentContext.mounted) {
        _showError(l10n.t('unbind_progress_failed', [e.toString()]));
      }
      return false;
    } finally {
      progressValue.dispose();
      progressText.dispose();
      if (currentContext.mounted) {
        if (!dialogPopped) {
          Navigator.of(currentContext, rootNavigator: true).pop();
          // Ignore any extra pops if the dialog was already dismissed.
        }
      }
      setState(() => _busy = false);
    }
  }

  Future<void> _removeLocalOnly() async {
    final l10n = AppLocalizations.of(context);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.t('remove_local_record')),
        content: Text(l10n.t('remove_local_record_confirm')),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.t('cancel')),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.t('delete')),
          ),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    await _deviceStorage.delete(_device.deviceId);
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(l10n.t('remove_local_record_done'))));
    Navigator.of(context).pop();
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
          if (!_isPeerShadow)
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
                if (ok != true) return;
                if (!mounted) return;
                final success = await _unbindWithProgress();
                if (!mounted) return;
                if (success) {
                  navigator.pop();
                }
              },
            ),
          IconButton(
            icon: const Icon(Icons.delete_outline),
            onPressed: _busy ? null : _removeLocalOnly,
            tooltip: l10n.t('remove_local_record'),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (_isPeerShadow)
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Colors.orange.withAlpha((0.1 * 255).round()),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  l10n.t('peer_device_readonly'),
                  style: TextStyle(color: Colors.orange.shade800),
                ),
              ),
            ),
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
          if (d.pairedWithDeviceId != null) ...[
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
              title: Text(l10n.t('triggered_by_pair_count')),
              subtitle: Text(l10n.t('triggered_by_pair_count_value', [d.triggeredByPairCount.toString()])),
            ),
            if (!_isPeerShadow)
              Padding(
                padding: const EdgeInsets.only(left: 16, right: 16, bottom: 8),
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.link_off, size: 18),
                  label: Text(l10n.t('unpair')),
                  onPressed: _busy ? null : _unpair,
                ),
              ),
          ],
          ListTile(
            title: const Text('IP'),
            subtitle: Text(d.ipAddress.isEmpty ? '—' : d.ipAddress),
          ),
          ListTile(
            title: Text(l10n.t('status')),
            subtitle: Text(
              _statusText(l10n, d),
            ),
          ),
          if (d.lastConnectedSsid != null && d.lastConnectedSsid!.isNotEmpty)
            ListTile(
              title: Text(l10n.t('wifi')),
              subtitle: Text(d.lastConnectedSsid!),
            ),
          if (!_isPeerShadow) ...[
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
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: _busy ? null : _removeLocalOnly,
            icon: const Icon(Icons.delete_outline, size: 18),
            label: Text(l10n.t('remove_local_record')),
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
    if (_isPeerShadow) {
      _showError(l10n.t('peer_device_readonly'));
      return;
    }
    if (_device.ipAddress.isEmpty) {
      _showError(l10n.t('device_ip_empty'));
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.moveServo(_device.ipAddress, index, angle);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(DeviceProvisioningService.pickErrorMessage(res) ?? l10n.t('set_failed'));
    }
  }

  Future<void> _demoServo() async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null) return;
    if (_isPeerShadow) {
      _showError(l10n.t('peer_device_readonly'));
      return;
    }
    if (_device.ipAddress.isEmpty) {
      _showError(l10n.t('device_ip_empty'));
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.demoServo(_device.ipAddress);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(DeviceProvisioningService.pickErrorMessage(res) ?? l10n.t('demo_failed'));
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

  Future<void> _unpair() async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null || _device.pairedWithDeviceId == null) return;
    if (_isPeerShadow) {
      _showError(l10n.t('peer_device_readonly'));
      return;
    }
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.t('unpair_title')),
        content: Text(l10n.t('unpair_confirm')),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.t('cancel'))),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: Text(l10n.t('unpair'))),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    final peerId = _device.pairedWithDeviceId!;
    final peer = await _deviceStorage.getByDeviceId(peerId);
    // Prefer the latest UDP-discovered IP to avoid stale/empty stored IP.
    final peerIpFromDiscovery = DiscoveredDevicesStore.getIp(peerId);
    final peerIp = (peerIpFromDiscovery != null && peerIpFromDiscovery.isNotEmpty)
        ? peerIpFromDiscovery
        : peer?.ipAddress;
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.unpair(_device.ipAddress, peerIp: peerIp);
    if (!mounted) return;
    if (res['status'] != 'ok') {
      setState(() => _busy = false);
      _showError(
        DeviceProvisioningService.pickErrorMessage(res) ??
            l10n.t('unpair_failed'),
      );
      return;
    }
    await _deviceStorage.delete(peerId);
    final updated = _device.copyWith(pairedWithDeviceId: null, triggeredByPairCount: 0);
    await _deviceStorage.save(updated);
    setState(() {
      _device = updated;
      _busy = false;
    });
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('unpair'))));
      Navigator.of(context).pop(updated);
    }
  }

  Future<void> _sendPairRequest() async {
    final l10n = AppLocalizations.maybeOf(context);
    if (l10n == null) return;
    if (_isPeerShadow) {
      _showError(l10n.t('peer_device_readonly'));
      return;
    }
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
    final list = await _wifiStorage.getAll();
    if (list.isNotEmpty) {
      final networks = list.map((w) => {'ssid': w.ssid, 'pwd': w.password, 'sec': w.securityType}).toList();
      await DeviceDiscoveryService.updateWifiList(_device.ipAddress, networks);
    }
    if (!mounted) return;
    final res = await DeviceDiscoveryService.pairRequest(_device.ipAddress, targetId, targetIp);
    if (!mounted) return;
    if (res['status'] != 'ok') {
      setState(() => _busy = false);
      _showError(
        DeviceProvisioningService.pickErrorMessage(res) ?? l10n.t('pair_request_failed'),
      );
      return;
    }
    final result = await _showPairWaitingDialog(targetId);
    if (!mounted) return;
    setState(() => _busy = false);
    if (result == null) return;
    final (peerId, peerIp) = result;
    await _deviceStorage.save(_device.copyWith(pairedWithDeviceId: peerId));
    final peerDevice = Device(
      deviceId: peerId,
      name: peerId,
      ipAddress: peerIp,
      isBound: true,
      pairedWithDeviceId: _device.deviceId,
      isPeerShadow: true,
    );
    await _deviceStorage.save(peerDevice);
    DiscoveredDevicesStore.update(peerId, peerIp);
    setState(() => _device = _device.copyWith(pairedWithDeviceId: peerId));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.t('pair_success'))));
  }

  Future<(String, String)?> _showPairWaitingDialog(String targetId) async {
    final l10n = AppLocalizations.of(context);
    final targetIp = DiscoveredDevicesStore.getIp(targetId) ?? '';
    final myDeviceIp = _device.ipAddress;
    final result = await showDialog<(String, String)?>(
      context: context,
      barrierDismissible: false,
      builder: (_) => _PairWaitingDialog(
        targetId: targetId,
        targetIp: targetIp,
        myDeviceIp: myDeviceIp,
      ),
    );
    if (result == null && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.t('pair_expired_or_rejected'))),
      );
    }
    return result;
  }

  static String _formatLastSeen(DateTime t) {
    final now = DateTime.now();
    if (t.year == now.year && t.month == now.month && t.day == now.day) {
      return '${t.hour.toString().padLeft(2, '0')}:${t.minute.toString().padLeft(2, '0')}';
    }
    return '${t.month.toString().padLeft(2, '0')}-${t.day.toString().padLeft(2, '0')} '
        '${t.hour.toString().padLeft(2, '0')}:${t.minute.toString().padLeft(2, '0')}';
  }
}

class _PairWaitingDialog extends StatefulWidget {
  const _PairWaitingDialog({
    required this.targetId,
    required this.targetIp,
    required this.myDeviceIp,
  });

  final String targetId;
  final String targetIp;
  final String myDeviceIp;

  @override
  State<_PairWaitingDialog> createState() => _PairWaitingDialogState();
}

class _PairWaitingDialogState extends State<_PairWaitingDialog> {
  static const _pollInterval = Duration(seconds: 2);
  static const _timeout = Duration(seconds: 90);
  DateTime? _deadline;

  @override
  void initState() {
    super.initState();
    _deadline = DateTime.now().add(_timeout);
    _poll();
  }

  Future<void> _poll() async {
    while (mounted && _deadline != null && DateTime.now().isBefore(_deadline!)) {
      await Future.delayed(_pollInterval);
      if (!mounted) return;
      final status = await DeviceDiscoveryService.getPairStatus(widget.myDeviceIp);
      if (!mounted) return;
      final pairedWith = (status['paired_with'] as String? ?? '').trim();
      final peerIpFromStatus = (status['peer_ip'] as String? ?? '').trim();
      if (pairedWith == widget.targetId) {
        final effectivePeerIp = peerIpFromStatus.isNotEmpty
            ? peerIpFromStatus
            : widget.targetIp;
        if (mounted) {
          Navigator.of(context).pop((widget.targetId, effectivePeerIp));
        }
        return;
      }
    }
    if (mounted) Navigator.of(context).pop<(String, String)?>(null);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(l10n.t('pair_waiting_title')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Padding(
            padding: EdgeInsets.all(16),
            child: SizedBox(
              width: 40,
              height: 40,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
          Text(l10n.t('pair_waiting_message', [widget.targetId])),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop<(String, String)?>(null),
          child: Text(l10n.t('cancel')),
        ),
      ],
    );
  }
}
