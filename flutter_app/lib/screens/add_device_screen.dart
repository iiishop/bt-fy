import 'package:flutter/material.dart';
import 'package:op_wifi_utils/op_wifi_utils.dart';
import 'package:wifi_scan/wifi_scan.dart';
import 'package:uuid/uuid.dart';

import '../main.dart';
import '../models/device.dart';
import '../models/wifi_network.dart';
import '../services/device_discovery_service.dart';
import '../services/device_provisioning_service.dart';
import '../services/device_storage_service.dart';
import '../services/pending_bind_store.dart';
import '../services/wifi_storage_service.dart';

/// 添加设备：扫描 ESP_ 热点 → 点击后弹窗确认 → 由应用连接热点 → 检测已连上后自动 identify → 选 Wi-Fi 发 config → 设备连路由器并关 SoftAP → 监听广播 → 确认绑定
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
        final exists = _pendingDevices.any((d) => d.deviceId == device.deviceId);
        if (!exists) _pendingDevices = [..._pendingDevices, device];
      });
    },
  );
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  final WifiStorageService _wifiStorage = WifiStorageService();

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

  @override
  void initState() {
    super.initState();
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
    final esp = list.where((ap) => ap.ssid.startsWith('ESP_')).toList();
    if (mounted) {
      setState(() {
        _apList = esp;
        _scanning = false;
      });
    }
  }

  /// 用户点击某个设备热点：弹窗确认 → 由应用连接该 SoftAP → 等待流量到达该热点 → 自动 identify
  Future<void> _onTapDeviceHotspot(String ssid) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('连接设备热点'),
        content: Text(
          '即将连接到此设备热点「$ssid」。\n\n'
          '确认后将由系统连接该热点，连接成功后会自动获取设备信息并进入下一步配网。',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('确认连接'),
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
            _connectError = '无法连接热点：$err';
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
          _connectError = '未检测到已连接至 $ssid，请到系统 Wi-Fi 中手动连接后重试';
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
              ? '识别设备失败：$identifyErr'
              : '已连接热点但无法识别设备，请确认设备已开机且固件支持配网';
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

  Future<void> _sendConfig(WifiNetwork wifi) async {
    setState(() {
      _configSending = true;
      _configError = null;
    });
    final bindToken = const Uuid().v4();
    final phoneId = const Uuid().v4();
    PendingBindStore.setPending(bindToken, phoneId);
    final result = await _provisioning.config(wifi: wifi, bindToken: bindToken);
    if (!mounted) return;
    setState(() {
      _configSending = false;
      final status = result['status'] as String?;
      if (status == 'connecting' || status == 'ok') {
        _configError = null;
        if (mounted) {
          showDialog(
            context: context,
            builder: (ctx) => AlertDialog(
              title: const Text('配置已发送'),
              content: const Text(
                '请切回家庭 Wi-Fi，设备连上路由器后会自动出现在首页列表中并完成绑定。',
              ),
              actions: [
                FilledButton(
                  onPressed: () {
                    Navigator.pop(ctx);
                    if (context.mounted) Navigator.pop(context);
                  },
                  child: const Text('确定'),
                ),
              ],
            ),
          );
        }
      } else {
        _configError = result['reason'] as String? ?? '配置失败';
        PendingBindStore.clear();
      }
    });
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
    final phoneId = const Uuid().v4();
    final result = await DeviceDiscoveryService.bind(d.ipAddress, phoneId);
    setState(() => _binding = false);
    if (!mounted) return;
    if (result['status'] == 'ok') {
      await _deviceStorage.save(d.copyWith(isBound: true, name: d.deviceId));
      if (mounted) Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('添加设备')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const Text('1. 扫描设备热点', style: TextStyle(fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          if (_scanning)
            const LinearProgressIndicator()
          else
            FilledButton(onPressed: _startScan, child: const Text('重新扫描')),
          if (_apList.isNotEmpty) ...[
            const SizedBox(height: 8),
            const Text('点击要添加的设备热点，由应用连接并自动识别：', style: TextStyle(color: Colors.grey, fontSize: 12)),
            const SizedBox(height: 4),
            ..._apList.map((ap) => ListTile(
                  title: Text(ap.ssid),
                  subtitle: Text('信号 ${ap.level} dBm · 点击后弹窗确认连接'),
                  enabled: !_connecting,
                  onTap: () => _onTapDeviceHotspot(ap.ssid),
                )),
          ],
          if (_connecting) ...[
            const SizedBox(height: 12),
            const Center(child: CircularProgressIndicator()),
            const Center(child: Text('正在连接设备热点并识别…')),
          ],
          if (_connectError != null) ...[
            const SizedBox(height: 8),
            Text(_connectError!, style: const TextStyle(color: Colors.red)),
          ],
          const Divider(height: 24),
          const Text('2. 设备信息', style: TextStyle(fontWeight: FontWeight.bold)),
          if (_identifiedDeviceId != null)
            ListTile(
              title: const Text('已发现设备'),
              subtitle: Text('$_identifiedDeviceId · 固件 $_identifiedFw'),
            )
          else if (!_connecting)
            const Text('点击上方设备热点，确认连接后会自动识别', style: TextStyle(color: Colors.grey)),
          if (_identifiedDeviceId != null) ...[
            const SizedBox(height: 8),
            const Text('选择家庭 Wi-Fi 并发送配置（发完后可切回家庭 Wi-Fi）', style: TextStyle(fontWeight: FontWeight.bold)),
            FutureBuilder<List<WifiNetwork>>(
              future: _wifiStorage.getAll(),
              builder: (context, snap) {
                final list = snap.data ?? [];
                if (list.isEmpty) {
                  return const Text('请先在「Wi-Fi 管理」中添加家庭 Wi-Fi', style: TextStyle(color: Colors.grey));
                }
                return Column(
                  children: list
                      .map((w) => ListTile(
                            title: Text(w.ssid),
                            onTap: _configSending ? null : () => _sendConfig(w),
                            trailing: _configSending
                                ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
                                : null,
                          ))
                      .toList(),
                );
              },
            ),
            if (_configError != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_configError!, style: const TextStyle(color: Colors.red)),
              ),
          ],
          if (_pendingDevices.isNotEmpty) ...[
            const Divider(height: 24),
            const Text('3. 待绑定设备（请确认舵机动作后绑定）', style: TextStyle(fontWeight: FontWeight.bold)),
            ..._pendingDevices.map((d) => ListTile(
                  title: Text(d.deviceId),
                  subtitle: Text(d.ipAddress),
                  trailing: _selectedPending?.deviceId == d.deviceId ? const Icon(Icons.check) : null,
                  onTap: () async {
                    setState(() => _selectedPending = d);
                    await DeviceDiscoveryService.demoServo(d.ipAddress);
                  },
                )),
            if (_selectedPending != null)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: FilledButton(
                  onPressed: _binding ? null : _confirmBind,
                  child: _binding
                      ? const SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2))
                      : const Text('确认绑定'),
                ),
              ),
          ],
        ],
      ),
    );
  }
}
