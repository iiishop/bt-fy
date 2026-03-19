import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

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
    list.sort((a, b) => a.ssid.toLowerCase().compareTo(b.ssid.toLowerCase()));
    setState(() {
      _list = list;
      _loading = false;
    });
  }

  Future<void> _addOrEdit([WifiNetwork? existing]) async {
    final l10n = AppLocalizations.of(context);
    final ssidController = TextEditingController(text: existing?.ssid ?? '');
    final pwdController = TextEditingController(text: existing?.password ?? '');
    bool obscure = true;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: Text(existing == null ? l10n.t('add_wifi') : l10n.t('edit_wifi')),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: ssidController,
                  decoration: const InputDecoration(
                    labelText: 'SSID',
                    prefixIcon: Icon(Icons.wifi),
                  ),
                  enabled: existing == null,
                  textInputAction: TextInputAction.next,
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: pwdController,
                  decoration: InputDecoration(
                    labelText: l10n.t('password'),
                    prefixIcon: const Icon(Icons.lock_outline),
                    suffixIcon: IconButton(
                      onPressed: () {
                        setDialogState(() => obscure = !obscure);
                      },
                      icon: Icon(
                        obscure ? Icons.visibility : Icons.visibility_off,
                      ),
                    ),
                  ),
                  obscureText: obscure,
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: Text(l10n.t('cancel')),
            ),
            FilledButton.icon(
              onPressed: () => Navigator.pop(ctx, true),
              icon: const Icon(Icons.save_outlined),
              label: Text(l10n.t('save')),
            ),
          ],
        ),
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
      body: AnimatedSwitcher(
        duration: const Duration(milliseconds: 220),
        child: _loading
            ? const Center(
                key: ValueKey('loading'),
                child: CircularProgressIndicator(),
              )
            : _list.isEmpty
            ? Center(
                key: const ValueKey('empty'),
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.wifi_find_outlined,
                        size: 72,
                        color: Colors.blueGrey.shade300,
                      ),
                      const SizedBox(height: 12),
                      Text(
                        l10n.t('no_wifi_saved'),
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodyLarge,
                      ),
                    ],
                  ),
                ),
              )
            : ListView.separated(
                key: const ValueKey('list'),
                padding: const EdgeInsets.fromLTRB(12, 12, 12, 92),
                itemCount: _list.length,
                separatorBuilder: (_, __) => const SizedBox(height: 8),
                itemBuilder: (context, index) {
                  final w = _list[index];
                  return AnimatedContainer(
                    duration: const Duration(milliseconds: 180),
                    curve: Curves.easeOutCubic,
                    child: Card(
                      margin: EdgeInsets.zero,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(14),
                      ),
                      child: ListTile(
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 4,
                        ),
                        leading: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            color: Colors.indigo.withAlpha((0.10 * 255).round()),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: const Icon(Icons.wifi, color: Colors.indigo),
                        ),
                        title: Text(
                          w.ssid,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: Text(l10n.t('password_saved')),
                        trailing: Wrap(
                          spacing: 4,
                          children: [
                            IconButton(
                              icon: const Icon(Icons.edit_outlined),
                              tooltip: l10n.t('edit_wifi'),
                              onPressed: () async {
                                await HapticFeedback.selectionClick();
                                _addOrEdit(w);
                              },
                            ),
                            IconButton(
                              icon: const Icon(Icons.delete_outline),
                              tooltip: l10n.t('delete'),
                              onPressed: () async {
                                await HapticFeedback.selectionClick();
                                _delete(w);
                              },
                            ),
                          ],
                        ),
                      ),
                    ),
                  );
                },
              ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () async {
          await HapticFeedback.mediumImpact();
          _addOrEdit();
        },
        child: const Icon(Icons.add),
      ),
    );
  }
}
