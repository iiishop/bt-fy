import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:op_wifi_utils/op_wifi_utils.dart';
import 'package:wifi_scan/wifi_scan.dart';
import 'package:uuid/uuid.dart';

import '../l10n/app_localizations.dart';
import '../main.dart';
import '../models/device.dart';
import '../models/wifi_network.dart';
import '../services/device_discovery_service.dart';
import '../services/device_provisioning_service.dart';
import '../services/phone_identity_service.dart';
import '../services/device_storage_service.dart';
import '../services/pending_bind_store.dart';
import '../services/wifi_storage_service.dart';

/// 添加设备：扫描 BF_ 热点 → 点击后弹窗确认 → 由应用连接热点 → 检测已连上后自动 identify → 选 Wi-Fi 发 config → 设备连路由器并关 SoftAP → 监听广播 → 确认绑定
class AddDeviceScreen extends StatefulWidget {
  const AddDeviceScreen({super.key});

  @override
  State<AddDeviceScreen> createState() => _AddDeviceScreenState();
}

class _AddDeviceScreenState extends State<AddDeviceScreen> {
  final DeviceProvisioningService _provisioning = DeviceProvisioningService();
  late final DeviceDiscoveryService _discovery = DeviceDiscoveryService(
    onDeviceSeen: (device) {
      if (!mounted) return;
      setState(() {
        final exists = _pendingDevices.any(
          (d) => d.deviceId == device.deviceId,
        );
        if (!exists) _pendingDevices = [..._pendingDevices, device];
      });
    },
    onBindingSeen: (deviceId, ip, bindToken) {
      unawaited(_handleBindingSeen(deviceId, ip, bindToken));
    },
  );
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  final WifiStorageService _wifiStorage = WifiStorageService();
  final PhoneIdentityService _phoneIdentity = PhoneIdentityService();

  List<WiFiAccessPoint> _apList = [];
  bool _scanning = false;
  bool _connecting = false;
  String? _connectError;
  String? _identifiedDeviceId;
  String? _identifiedFw;
  bool _configSending = false;
  String? _configError;
  List<Device> _pendingDevices = [];
  Device? _selectedPending;
  bool _binding = false;
  bool _waitingBindingAfterConfig = false;

  @override
  void initState() {
    super.initState();
    _discovery.startListening();
    _startScan();
  }

  Future<void> _startScan() async {
    setState(() {
      _apList = [];
      _scanning = true;
      _connectError = null;
      _identifiedDeviceId = null;
      _identifiedFw = null;
      _configError = null;
      _pendingDevices = [];
      _selectedPending = null;
    });
    final canStart = await WiFiScan.instance.canStartScan(askPermissions: true);
    if (canStart == CanStartScan.yes) {
      await WiFiScan.instance.startScan();
      await Future.delayed(const Duration(seconds: 3));
    }
    final list = await WiFiScan.instance.getScannedResults();
    final esp = list.where((ap) => ap.ssid.startsWith('BF_')).toList();
    if (mounted) {
      setState(() {
        _apList = esp;
        _scanning = false;
      });
    }
  }

  Future<void> _onTapDeviceHotspot(String ssid) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.t('connect_ap_title')),
        content: Text(l10n.t('connect_ap_message', [ssid])),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.t('cancel')),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.t('confirm_connect')),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    setState(() {
      _connecting = true;
      _connectError = null;
      _identifiedDeviceId = null;
      _identifiedFw = null;
    });

    try {
      // 1. 由应用发起连接（设备热点为开放网络，无密码）
      appLog.i('连接设备热点: $ssid');
      final connectResult = await OpWifiUtils.connectToWifi(ssid: ssid);
      if (!connectResult.isSuccess) {
        final err = connectResult.error.type.toString();
        appLog.w('connectToWifi 失败: $err');
        if (mounted) {
          setState(() {
            _connecting = false;
            _connectError = l10n.t('connect_error', [err]);
          });
        }
        return;
      }

      // 2. 等待当前 WiFi 已是该热点（确保流量到达该 SoftAP）
      const maxWait = Duration(seconds: 25);
      const step = Duration(milliseconds: 800);
      var elapsed = Duration.zero;
      String? currentSsid;
      while (elapsed < maxWait && mounted) {
        await Future.delayed(step);
        elapsed += step;
        final ssidResult = await OpWifiUtils.getCurrentSsid();
        if (ssidResult.isSuccess) {
          currentSsid = ssidResult.data;
          if (currentSsid == ssid) break;
        }
      }

      if (!mounted) return;
      if (currentSsid != ssid) {
        setState(() {
          _connecting = false;
          _connectError = l10n.t('not_connected_to_ap', [ssid]);
        });
        return;
      }

      appLog.i('已连接至 $ssid，等待网络就绪后 identify');
      // 3. 等待路由/DHCP 完全就绪后再发起 identify（手机刚拿到 192.168.71.2 后稍等）
      await Future.delayed(const Duration(milliseconds: 1500));
      String? identifyErr;
      final deviceId = await _provisioning.identify(
        onError: (msg) {
          identifyErr = msg;
          appLog.w('identify onError: $msg');
        },
      );
      if (!mounted) return;
      if (deviceId == null) {
        setState(() {
          _connecting = false;
          _connectError = identifyErr != null
              ? l10n.t('identify_failed', [identifyErr!])
              : l10n.t('identify_failed_generic');
        });
        return;
      }
      setState(() {
        _connecting = false;
        _connectError = null;
        _identifiedDeviceId = deviceId['deviceId'] as String?;
        _identifiedFw = deviceId['fw'] as String?;
      });
    } catch (e, st) {
      appLog.e('连接/识别异常', error: e, stackTrace: st);
      if (mounted) {
        setState(() {
          _connecting = false;
          _connectError = e.toString();
        });
      }
    }
  }

  Future<void> _sendConfigAll() async {
    final list = await _wifiStorage.getAll();
    if (list.isEmpty) return;
    setState(() {
      _configSending = true;
      _configError = null;
      _waitingBindingAfterConfig = false;
    });
    final bindToken = const Uuid().v4();
    String phoneId;
    try {
      phoneId = await _phoneIdentity.getStablePhoneId();
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _configSending = false;
        _configError = AppLocalizations.of(
          context,
        ).t('stable_phone_id_required');
      });
      return;
    }
    await PendingBindStore.setPending(bindToken, phoneId);
    final result = await _provisioning.config(
      networks: list,
      bindToken: bindToken,
    );
    if (!mounted) return;
    setState(() {
      _configSending = false;
      final status = result['status'] as String?;
      if (status == 'connecting' || status == 'ok') {
        _configError = null;
        _waitingBindingAfterConfig = true;
      } else {
        _configError =
            DeviceProvisioningService.pickErrorMessage(result) ??
            AppLocalizations.maybeOf(context)?.t('config_failed') ??
            'Config failed';
        _waitingBindingAfterConfig = false;
        unawaited(PendingBindStore.clear());
      }
    });
    if (mounted && _waitingBindingAfterConfig) {
      final l10n = AppLocalizations.of(context);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.t('bind_waiting_message'))));
    }
  }

  Future<void> _handleBindingSeen(
    String deviceId,
    String ip,
    String bindToken,
  ) async {
    if (!mounted || ip.isEmpty || ip == '0.0.0.0') return;
    final pending = await PendingBindStore.getPending();
    if (pending == null || pending.token != bindToken) return;

    final bindRes = await DeviceDiscoveryService.bind(ip, pending.phoneId);
    if (bindRes['status'] != 'ok') return;

    await PendingBindStore.clear();
    await _deviceStorage.save(
      Device(
        deviceId: deviceId,
        name: deviceId,
        ipAddress: ip,
        isBound: true,
        boundPhoneId: pending.phoneId,
      ),
    );
    if (!mounted) return;
    setState(() {
      _waitingBindingAfterConfig = false;
      _configError = null;
    });
    final l10n = AppLocalizations.of(context);
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(l10n.t('bind_success_message'))));
    Navigator.pop(context);
  }

  @override
  void dispose() {
    _discovery.stopListening();
    super.dispose();
  }

  Future<void> _confirmBind() async {
    final d = _selectedPending;
    if (d == null) return;
    setState(() => _binding = true);
    String phoneId;
    try {
      phoneId = await _phoneIdentity.getStablePhoneId();
    } catch (_) {
      if (!mounted) return;
      setState(() => _binding = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            AppLocalizations.of(context).t('stable_phone_id_required'),
          ),
        ),
      );
      return;
    }
    final result = await DeviceDiscoveryService.bind(d.ipAddress, phoneId);
    setState(() => _binding = false);
    if (!mounted) return;
    if (result['status'] == 'ok') {
      await _deviceStorage.save(
        d.copyWith(isBound: true, name: d.deviceId, boundPhoneId: phoneId),
      );
      if (mounted) Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.t('add_device_title'))),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _StepSectionCard(
            stepLabel: l10n.t('step1_title'),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                AnimatedSwitcher(
                  duration: const Duration(milliseconds: 220),
                  child: _scanning
                      ? const LinearProgressIndicator(key: ValueKey('scanning'))
                      : FilledButton.icon(
                          key: const ValueKey('rescan'),
                          onPressed: () async {
                            await HapticFeedback.selectionClick();
                            _startScan();
                          },
                          icon: const Icon(Icons.radar),
                          label: Text(l10n.t('rescan')),
                        ),
                ),
                if (_apList.isNotEmpty) ...[
                  const SizedBox(height: 10),
                  Text(
                    l10n.t('step1_subtitle'),
                    style: const TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                  const SizedBox(height: 6),
                  ..._apList.map(
                    (ap) => Card(
                      margin: const EdgeInsets.only(bottom: 8),
                      child: ListTile(
                        leading: const Icon(Icons.wifi),
                        title: Text(ap.ssid),
                        subtitle: Text(
                          l10n.t('step1_ap_subtitle', [ap.level.toString()]),
                        ),
                        enabled: !_connecting,
                        onTap: () async {
                          await HapticFeedback.selectionClick();
                          _onTapDeviceHotspot(ap.ssid);
                        },
                      ),
                    ),
                  ),
                ],
                if (_connecting) ...[
                  const SizedBox(height: 12),
                  const Center(child: CircularProgressIndicator()),
                  const SizedBox(height: 8),
                  Center(child: Text(l10n.t('connecting'))),
                ],
                if (_connectError != null) ...[
                  const SizedBox(height: 8),
                  _InlineInfoBanner(
                    icon: Icons.error_outline,
                    text: _connectError!,
                    tone: _BannerTone.error,
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 12),
          _StepSectionCard(
            stepLabel: l10n.t('step2_title'),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                AnimatedSwitcher(
                  duration: const Duration(milliseconds: 220),
                  child: _identifiedDeviceId != null
                      ? ListTile(
                          key: const ValueKey('identified'),
                          contentPadding: EdgeInsets.zero,
                          leading: const Icon(Icons.memory),
                          title: Text(l10n.t('device_found')),
                          subtitle: Text(
                            '$_identifiedDeviceId · ${l10n.t('firmware')} $_identifiedFw',
                          ),
                        )
                      : Text(
                          l10n.t('step2_subtitle'),
                          key: const ValueKey('not-identified'),
                          style: const TextStyle(color: Colors.grey),
                        ),
                ),
                if (_identifiedDeviceId != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    l10n.t('step2_wifi_hint'),
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 6),
                  FutureBuilder<List<WifiNetwork>>(
                    future: _wifiStorage.getAll(),
                    builder: (context, snap) {
                      final list = snap.data ?? [];
                      if (list.isEmpty) {
                        return _InlineInfoBanner(
                          icon: Icons.info_outline,
                          text: l10n.t('add_wifi_first'),
                          tone: _BannerTone.neutral,
                        );
                      }
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          ...list.map(
                            (w) => Card(
                              margin: const EdgeInsets.only(bottom: 8),
                              child: ListTile(
                                leading: const Icon(Icons.wifi),
                                title: Text(w.ssid),
                              ),
                            ),
                          ),
                          const SizedBox(height: 8),
                          FilledButton.icon(
                            onPressed: _configSending
                                ? null
                                : () async {
                                    await HapticFeedback.mediumImpact();
                                    _sendConfigAll();
                                  },
                            icon: _configSending
                                ? const SizedBox(
                                    width: 16,
                                    height: 16,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                    ),
                                  )
                                : const Icon(Icons.send),
                            label: Text(l10n.t('send_config_all')),
                          ),
                        ],
                      );
                    },
                  ),
                  if (_configError != null) ...[
                    const SizedBox(height: 8),
                    _InlineInfoBanner(
                      icon: Icons.error_outline,
                      text: _configError!,
                      tone: _BannerTone.error,
                    ),
                  ],
                  if (_waitingBindingAfterConfig) ...[
                    const SizedBox(height: 8),
                    _InlineInfoBanner(
                      icon: Icons.hourglass_top_rounded,
                      text: l10n.t('bind_waiting_message'),
                      tone: _BannerTone.neutral,
                      loading: true,
                    ),
                  ],
                ],
              ],
            ),
          ),
          if (_pendingDevices.isNotEmpty) ...[
            const SizedBox(height: 12),
            _StepSectionCard(
              stepLabel: l10n.t('step3_title'),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  ..._pendingDevices.map(
                    (d) => Card(
                      margin: const EdgeInsets.only(bottom: 8),
                      child: ListTile(
                        leading: const Icon(Icons.developer_board),
                        title: Text(d.deviceId),
                        subtitle: Text(d.ipAddress),
                        trailing: _selectedPending?.deviceId == d.deviceId
                            ? const Icon(
                                Icons.check_circle,
                                color: Colors.green,
                              )
                            : const Icon(Icons.chevron_right),
                        onTap: () async {
                          await HapticFeedback.selectionClick();
                          setState(() => _selectedPending = d);
                          await DeviceDiscoveryService.demoServo(d.ipAddress);
                        },
                      ),
                    ),
                  ),
                  if (_selectedPending != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: FilledButton.icon(
                        onPressed: _binding
                            ? null
                            : () async {
                                await HapticFeedback.mediumImpact();
                                _confirmBind();
                              },
                        icon: _binding
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.verified_user),
                        label: Text(l10n.t('confirm_bind')),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

enum _BannerTone { neutral, error }

class _InlineInfoBanner extends StatelessWidget {
  const _InlineInfoBanner({
    required this.icon,
    required this.text,
    required this.tone,
    this.loading = false,
  });

  final IconData icon;
  final String text;
  final _BannerTone tone;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    final isError = tone == _BannerTone.error;
    final bg = isError
        ? Colors.red.withAlpha((0.08 * 255).round())
        : Colors.blueGrey.withAlpha((0.08 * 255).round());
    final fg = isError ? Colors.red.shade700 : Colors.blueGrey.shade700;
    return AnimatedContainer(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(10),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          loading
              ? SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2, color: fg),
                )
              : Icon(icon, size: 18, color: fg),
          const SizedBox(width: 8),
          Expanded(
            child: Text(text, style: TextStyle(color: fg)),
          ),
        ],
      ),
    );
  }
}

class _StepSectionCard extends StatelessWidget {
  const _StepSectionCard({required this.stepLabel, required this.child});

  final String stepLabel;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              stepLabel,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 10),
            child,
          ],
        ),
      ),
    );
  }
}
