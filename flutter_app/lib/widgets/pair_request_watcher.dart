import 'dart:async';

import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../services/device_discovery_service.dart';
import '../services/device_provisioning_service.dart';
import '../services/device_storage_service.dart';

class PairRequestWatcher extends StatefulWidget {
  const PairRequestWatcher({
    super.key,
    required this.navigatorKey,
    required this.child,
  });

  final GlobalKey<NavigatorState> navigatorKey;
  final Widget child;

  @override
  State<PairRequestWatcher> createState() => _PairRequestWatcherState();
}

class _PairRequestDialogData {
  const _PairRequestDialogData({
    required this.targetHost,
    required this.fromDeviceId,
    required this.fromIp,
  });

  final String targetHost;
  final String fromDeviceId;
  final String fromIp;
}

class _PairRequestWatcherState extends State<PairRequestWatcher> {
  final DeviceStorageService _deviceStorage = DeviceStorageService();

  static const _pollInterval = Duration(seconds: 4);
  Timer? _timer;
  bool _polling = false;
  bool _dialogOpen = false;
  int _roundRobinCursor = 0;

  final ValueNotifier<_PairRequestDialogData?> _pendingNotifier =
      ValueNotifier<_PairRequestDialogData?>(null);

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(_pollInterval, (_) => _pollOnce());
    _pollOnce();
  }

  @override
  void dispose() {
    _timer?.cancel();
    _timer = null;
    _pendingNotifier.dispose();
    super.dispose();
  }

  Future<void> _pollOnce() async {
    if (_polling) return;
    _polling = true;
    try {
      final devices = await _deviceStorage.getAll();
      final currentTargetHost = _pendingNotifier.value?.targetHost;
      final candidates = (_dialogOpen && currentTargetHost != null)
          ? devices.where((d) => d.ipAddress == currentTargetHost).toList()
          : devices;
      if (candidates.isEmpty) return;
      final start = _roundRobinCursor % candidates.length;
      final iterDevices = [
        ...candidates.skip(start),
        ...candidates.take(start),
      ];
      _roundRobinCursor = (_roundRobinCursor + 1) % candidates.length;

      for (final d in iterDevices) {
        final host = d.ipAddress;
        if (host.isEmpty) continue;

        final res = await DeviceDiscoveryService.getPendingPairRequests(host);
        final pending = (res['pending'] as List<dynamic>?) ?? const [];
        if (pending.isEmpty) continue;

        final first = pending.first as Map<String, dynamic>;
        final fromDeviceId = (first['from_device_id'] as String?)?.trim() ?? '';
        final fromIp = (first['from_ip'] as String?)?.trim() ?? '';
        if (fromDeviceId.isEmpty) continue;

        _pendingNotifier.value = _PairRequestDialogData(
          targetHost: host,
          fromDeviceId: fromDeviceId,
          fromIp: fromIp,
        );

        if (!_dialogOpen) {
          if (!mounted) return;
          final ctx = widget.navigatorKey.currentContext;
          if (ctx == null) return;
          if (!ctx.mounted) return;
          _dialogOpen = true;

          unawaited(
            showDialog<void>(
              context: ctx,
              barrierDismissible: false,
              builder: (dialogCtx) {
                return ValueListenableBuilder<_PairRequestDialogData?>(
                  valueListenable: _pendingNotifier,
                  builder: (context, data, _) {
                    final l10n = AppLocalizations.of(context);
                    if (data == null) {
                      return AlertDialog(
                        title: Text(l10n.t('pair_request_title')),
                        content: const Text('...'),
                      );
                    }

                    return _PairRequestDialog(
                      data: data,
                      onAccept: () async {
                        final cur = _pendingNotifier.value;
                        if (cur == null) {
                          return {
                            'status': 'error',
                            'reason': 'no_pending_request',
                          };
                        }
                        return DeviceDiscoveryService.acceptPair(
                          cur.targetHost,
                          cur.fromDeviceId,
                        );
                      },
                      onReject: () async {
                        final cur = _pendingNotifier.value;
                        if (cur == null) {
                          return {
                            'status': 'error',
                            'reason': 'no_pending_request',
                          };
                        }
                        return DeviceDiscoveryService.rejectPair(
                          cur.targetHost,
                          cur.fromDeviceId,
                        );
                      },
                    );
                  },
                );
              },
            ).then((_) {
              _dialogOpen = false;
              _pendingNotifier.value = null;
            }),
          );
        }

        // Only handle one pending request at a time.
        return;
      }
    } finally {
      _polling = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}

class _PairRequestDialog extends StatefulWidget {
  const _PairRequestDialog({
    required this.data,
    required this.onAccept,
    required this.onReject,
  });

  final _PairRequestDialogData data;
  final Future<Map<String, dynamic>> Function() onAccept;
  final Future<Map<String, dynamic>> Function() onReject;

  @override
  State<_PairRequestDialog> createState() => _PairRequestDialogState();
}

class _PairRequestDialogState extends State<_PairRequestDialog> {
  bool _busy = false;
  String? _errorText;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final fromId = widget.data.fromDeviceId;
    final fromIp = widget.data.fromIp;

    return AlertDialog(
      title: Text(l10n.t('pair_request_title')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(l10n.t('pair_request_content', [fromId, fromIp])),
          if (_errorText != null) ...[
            const SizedBox(height: 10),
            Text(_errorText!, style: TextStyle(color: Colors.red.shade700)),
          ],
        ],
      ),
      actions: [
        TextButton(
          onPressed: _busy
              ? null
              : () async {
                  setState(() => _busy = true);
                  final result = await widget.onReject();
                  if (!context.mounted) return;
                  final ok = result['status'] == 'ok';
                  if (ok) {
                    Navigator.of(context).pop();
                  } else {
                    setState(() {
                      _busy = false;
                      _errorText =
                          DeviceProvisioningService.pickErrorMessage(result) ??
                          l10n.t('pair_action_failed');
                    });
                  }
                },
          child: Text(l10n.t('reject')),
        ),
        FilledButton(
          onPressed: _busy
              ? null
              : () async {
                  setState(() => _busy = true);
                  final result = await widget.onAccept();
                  if (!context.mounted) return;
                  final ok = result['status'] == 'ok';
                  if (ok) {
                    Navigator.of(context).pop();
                  } else {
                    setState(() {
                      _busy = false;
                      _errorText =
                          DeviceProvisioningService.pickErrorMessage(result) ??
                          l10n.t('pair_action_failed');
                    });
                  }
                },
          child: Text(l10n.t('accept')),
        ),
      ],
    );
  }
}
