import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:logger/logger.dart';

import 'l10n/app_localizations.dart';
import 'services/phone_identity_service.dart';
import 'screens/home_screen.dart';
import 'widgets/pair_request_watcher.dart';

final appLog = Logger(
  printer: PrettyPrinter(methodCount: 0, lineLength: 80),
  level: kReleaseMode ? Level.off : Level.debug,
  output: _DebugPrintOutput(),
);

class _DebugPrintOutput extends LogOutput {
  @override
  void output(OutputEvent event) {
    for (final line in event.lines) {
      debugPrint(line);
    }
  }

  @override
  Future<void> init() async {}

  @override
  Future<void> destroy() async {}
}

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  debugPrint('Smart Devices app starting');
  appLog.i('Logger ready');
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({super.key});

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  String _locale = 'en';
  final GlobalKey<NavigatorState> _navigatorKey = GlobalKey<NavigatorState>();
  final PhoneIdentityService _phoneIdentity = PhoneIdentityService();
  bool _stableIdChecked = false;

  void _setLocale(String locale) {
    setState(() => _locale = locale);
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => _checkStableIdOnStartup(),
    );
  }

  Future<void> _checkStableIdOnStartup() async {
    if (_stableIdChecked) return;
    _stableIdChecked = true;
    try {
      await _phoneIdentity.getStablePhoneId();
    } catch (_) {
      final ctx = _navigatorKey.currentContext;
      if (ctx == null || !mounted) return;
      final l10n = AppLocalizations.of(ctx);
      await showDialog<void>(
        context: ctx,
        builder: (dialogCtx) => AlertDialog(
          title: Text(l10n.t('device_info_title')),
          content: Text(l10n.t('stable_phone_id_required')),
          actions: [
            FilledButton(
              onPressed: () => Navigator.pop(dialogCtx),
              child: Text(l10n.t('ok')),
            ),
          ],
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return AppLocalizations(
      locale: _locale,
      setLocale: _setLocale,
      child: Builder(
        builder: (context) {
          final l10n = AppLocalizations.of(context);
          return MaterialApp(
            title: l10n.t('app_title'),
            navigatorKey: _navigatorKey,
            theme: ThemeData(
              colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
              useMaterial3: true,
            ),
            home: PairRequestWatcher(
              navigatorKey: _navigatorKey,
              child: const HomeScreen(),
            ),
          );
        },
      ),
    );
  }
}
