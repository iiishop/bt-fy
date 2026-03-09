import 'package:flutter/material.dart';

import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_storage_service.dart';

/// 设备详情：舵机控制、解绑（设计 2.4）
class DeviceDetailScreen extends StatefulWidget {
  const DeviceDetailScreen({super.key, required this.device});

  final Device device;

  @override
  State<DeviceDetailScreen> createState() => _DeviceDetailScreenState();
}

class _DeviceDetailScreenState extends State<DeviceDetailScreen> {
  final DeviceStorageService _deviceStorage = DeviceStorageService();
  int _servo0Angle = 90;
  int _servo1Angle = 90;
  bool _busy = false;

  @override
  Widget build(BuildContext context) {
    final d = widget.device;
    return Scaffold(
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
                  title: const Text('解绑设备'),
                  content: const Text('确定要解绑该设备吗？'),
                  actions: [
                    TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
                    FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('解绑')),
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
            title: const Text('设备 ID'),
            subtitle: Text(d.deviceId),
          ),
          ListTile(
            title: const Text('IP'),
            subtitle: Text(d.ipAddress.isEmpty ? '—' : d.ipAddress),
          ),
          const Divider(height: 24),
          const Text('舵机 0', style: TextStyle(fontWeight: FontWeight.bold)),
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
                child: const Text('设置角度'),
              ),
              const SizedBox(width: 16),
              OutlinedButton(
                onPressed: _busy ? null : _demoServo,
                child: const Text('演示 40°→120°→40°'),
              ),
            ],
          ),
          const SizedBox(height: 16),
          const Text('舵机 1', style: TextStyle(fontWeight: FontWeight.bold)),
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
              child: const Text('设置角度'),
            ),
          ),
        ],
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
    if (widget.device.ipAddress.isEmpty) {
      _showError('设备 IP 为空，请返回首页等待设备上线');
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.moveServo(widget.device.ipAddress, index, angle);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(res['reason']?.toString() ?? '设置失败');
    }
  }

  Future<void> _demoServo() async {
    if (widget.device.ipAddress.isEmpty) {
      _showError('设备 IP 为空，请返回首页等待设备上线');
      return;
    }
    setState(() => _busy = true);
    final res = await DeviceDiscoveryService.demoServo(widget.device.ipAddress);
    if (!mounted) return;
    setState(() => _busy = false);
    if (res['status'] != 'ok') {
      _showError(res['reason']?.toString() ?? '演示失败');
    }
  }
}
