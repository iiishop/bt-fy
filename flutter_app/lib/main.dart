import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:logger/logger.dart';

import 'l10n/app_localizations.dart';
import 'screens/home_screen.dart';

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

  void _setLocale(String locale) {
    setState(() => _locale = locale);
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
            theme: ThemeData(
              colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
              useMaterial3: true,
            ),
            home: const HomeScreen(),
          );
        },
      ),
    );
  }
}
