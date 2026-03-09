import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../models/wifi_network.dart';
import '../services/wifi_storage_service.dart';

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
    final l10n = AppLocalizations.of(context);
    final ssidController = TextEditingController(text: existing?.ssid ?? '');
    final pwdController = TextEditingController(text: existing?.password ?? '');
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(existing == null ? l10n.t('add_wifi') : l10n.t('edit_wifi')),
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
                decoration: InputDecoration(labelText: l10n.t('password')),
                obscureText: true,
              ),
            ],
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.t('cancel'))),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.t('save')),
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
    final l10n = AppLocalizations.of(context);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.t('delete')),
        content: Text(l10n.t('delete_confirm', [w.ssid])),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.t('cancel'))),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: Text(l10n.t('delete'))),
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
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.t('wifi_manage_title'))),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _list.isEmpty
              ? Center(child: Text(l10n.t('no_wifi_saved')))
              : ListView.builder(
                  itemCount: _list.length,
                  itemBuilder: (context, index) {
                    final w = _list[index];
                    return ListTile(
                      title: Text(w.ssid),
                      subtitle: Text(l10n.t('password_saved')),
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
