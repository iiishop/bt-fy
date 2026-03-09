import 'package:flutter/material.dart';

import '../models/wifi_network.dart';
import '../services/wifi_storage_service.dart';

/// Wi-Fi 网络管理：增删改查（设计 2.2）
class WifiManageScreen extends StatefulWidget {
  const WifiManageScreen({super.key});

  @override
  State<WifiManageScreen> createState() => _WifiManageScreenState();
}

class _WifiManageScreenState extends State<WifiManageScreen> {
  final WifiStorageService _storage = WifiStorageService();
  List<WifiNetwork> _list = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loading = true);
    final list = await _storage.getAll();
    setState(() {
      _list = list;
      _loading = false;
    });
  }

  Future<void> _addOrEdit([WifiNetwork? existing]) async {
    final ssidController = TextEditingController(text: existing?.ssid ?? '');
    final pwdController = TextEditingController(text: existing?.password ?? '');
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(existing == null ? '添加 Wi-Fi' : '编辑 Wi-Fi'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: ssidController,
                decoration: const InputDecoration(labelText: 'SSID'),
                enabled: existing == null,
              ),
              TextField(
                controller: pwdController,
                decoration: const InputDecoration(labelText: '密码'),
                obscureText: true,
              ),
            ],
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    if (ok == true && ssidController.text.trim().isNotEmpty) {
      await _storage.save(WifiNetwork(
        ssid: ssidController.text.trim(),
        password: pwdController.text,
        securityType: 3,
      ));
      _load();
    }
  }

  Future<void> _delete(WifiNetwork w) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除'),
        content: Text('确定删除「${w.ssid}」？'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('删除')),
        ],
      ),
    );
    if (ok == true) {
      await _storage.delete(w.ssid);
      _load();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Wi-Fi 管理')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _list.isEmpty
              ? const Center(child: Text('暂无已保存的 Wi-Fi，请点击 + 添加'))
              : ListView.builder(
                  itemCount: _list.length,
                  itemBuilder: (context, index) {
                    final w = _list[index];
                    return ListTile(
                      title: Text(w.ssid),
                      subtitle: Text('已保存密码'),
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(icon: const Icon(Icons.edit), onPressed: () => _addOrEdit(w)),
                          IconButton(icon: const Icon(Icons.delete), onPressed: () => _delete(w)),
                        ],
                      ),
                    );
                  },
                ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _addOrEdit(),
        child: const Icon(Icons.add),
      ),
    );
  }
}
