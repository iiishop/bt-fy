import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../services/phone_identity_service.dart';

class DeviceInfoScreen extends StatefulWidget {
  const DeviceInfoScreen({super.key});

  @override
  State<DeviceInfoScreen> createState() => _DeviceInfoScreenState();
}

class _DeviceInfoScreenState extends State<DeviceInfoScreen> {
  final PhoneIdentityService _identity = PhoneIdentityService();
  bool _loading = true;
  String? _stableId;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final id = await _identity.getStablePhoneId();
      if (!mounted) return;
      setState(() {
        _stableId = id;
        _loading = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _stableId = null;
        _loading = false;
        _error = AppLocalizations.of(context).t('stable_phone_id_required');
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.t('device_info_title')),
        actions: [
          IconButton(
            onPressed: _loading ? null : _load,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: _loading
            ? const Center(child: CircularProgressIndicator())
            : Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        l10n.t('device_info_phone_id'),
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 10),
                      SelectableText(
                        _stableId ?? _error ?? '-',
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 8,
                        children: [
                          FilledButton.icon(
                            onPressed: _stableId == null
                                ? null
                                : () async {
                                    await Clipboard.setData(
                                      ClipboardData(text: _stableId!),
                                    );
                                    if (!context.mounted) return;
                                    ScaffoldMessenger.of(context).showSnackBar(
                                      SnackBar(
                                        content: Text(
                                          l10n.t('copied_to_clipboard'),
                                        ),
                                      ),
                                    );
                                  },
                            icon: const Icon(Icons.copy),
                            label: Text(l10n.t('copy_id')),
                          ),
                          OutlinedButton.icon(
                            onPressed: _loading ? null : _load,
                            icon: const Icon(Icons.refresh),
                            label: Text(l10n.t('refresh')),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
      ),
    );
  }
}
