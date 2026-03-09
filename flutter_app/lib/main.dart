import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:logger/logger.dart';

import 'screens/home_screen.dart';

/// 全局 Logger。输出到 debugPrint，便于在 `flutter run` 终端看到。
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
  debugPrint('智能设备 App 启动');
  appLog.i('Logger 已就绪');
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '智能设备',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
        useMaterial3: true,
      ),
      home: const HomeScreen(),
    );
  }
}
