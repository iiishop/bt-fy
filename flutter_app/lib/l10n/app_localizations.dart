import 'package:flutter/material.dart';

import 'strings.dart';

/// Locale: 'en' or 'zh'. Exposed so app can switch language with one tap.
class AppLocalizations extends InheritedWidget {
  const AppLocalizations({
    super.key,
    required this.locale,
    required this.setLocale,
    required super.child,
  });

  final String locale;
  final void Function(String) setLocale;

  static AppLocalizations of(BuildContext context) {
    final w = context.dependOnInheritedWidgetOfExactType<AppLocalizations>();
    assert(w != null, 'No AppLocalizations above this context');
    return w!;
  }

  static AppLocalizations? maybeOf(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<AppLocalizations>();
  }

  Map<String, String> get _strings =>
      locale == 'zh' ? AppStrings.zh : AppStrings.en;

  /// Get localized string for [key]. Use %1, %2 in string and pass [params].
  String t(String key, [List<Object>? params]) {
    String s = _strings[key] ?? key;
    if (params != null) {
      for (var i = 0; i < params.length; i++) {
        s = s.replaceAll('%${i + 1}', params[i].toString());
      }
    }
    return s;
  }

  /// Current display label for language toggle (opposite of current).
  String get languageToggleLabel => locale == 'en' ? '中文' : 'EN';

  @override
  bool updateShouldNotify(AppLocalizations old) =>
      old.locale != locale;
}
