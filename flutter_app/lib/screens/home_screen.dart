import 'dart:async';

import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../models/device.dart';
import '../services/phone_identity_service.dart';
import '../viewmodels/home_view_model.dart';
import 'add_device_screen.dart';
import 'device_detail_screen.dart';
import 'device_info_screen.dart';
import 'wifi_manage_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  late final HomeViewModel _vm = HomeViewModel()..start();
  final PhoneIdentityService _phoneIdentity = PhoneIdentityService();

  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
    _vm.dispose();
    super.dispose();
  }

  String _statusText(AppLocalizations l10n, Device d) {
    switch (HomeViewModel.presenceOf(d)) {
      case DevicePresence.online:
        return l10n.t('currently_online');
      case DevicePresence.suspected:
        return l10n.t('suspected_offline', [_formatLastSeen(d.lastSeen)]);
      case DevicePresence.offline:
        return l10n.t('last_seen', [_formatLastSeen(d.lastSeen)]);
    }
  }

  (_Presence, String) _presenceAndStatus(AppLocalizations l10n, Device d) {
    final vmPresence = HomeViewModel.presenceOf(d);
    final presence = switch (vmPresence) {
      DevicePresence.online => _Presence.online,
      DevicePresence.suspected => _Presence.suspected,
      DevicePresence.offline => _Presence.offline,
    };
    return (presence, _statusText(l10n, d));
  }

  Future<void> _addDiscoveredDevice(Device d) async {
    final l10n = AppLocalizations.of(context);
    String phoneId;
    try {
      phoneId = await _phoneIdentity.getStablePhoneId();
    } catch (_) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.t('stable_phone_id_required'))),
      );
      return;
    }
    await _vm.addDiscoveredDevice(d, phoneId: phoneId);
  }

  Future<void> _openDeviceDetail(Device device) async {
    final l10n = AppLocalizations.of(context);
    if (!device.isPeerShadow) {
      final bound = device.boundPhoneId?.trim();
      if (bound == null || bound.isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.t('device_owner_unknown'))),
        );
        return;
      }
      String phoneId;
      try {
        phoneId = await _phoneIdentity.getStablePhoneId();
      } catch (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.t('stable_phone_id_required'))),
        );
        return;
      }
      if (bound != phoneId) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.t('device_owner_mismatch'))),
        );
        return;
      }
    }
    final updated = await Navigator.push<Device>(
      context,
      MaterialPageRoute(builder: (_) => DeviceDetailScreen(device: device)),
    );
    if (updated != null) {
      await _vm.upsertDevice(updated);
    }
    await _vm.loadDevices();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AnimatedBuilder(
      animation: _vm,
      builder: (context, _) => Scaffold(
        appBar: AppBar(
          title: Text(l10n.t('app_title')),
          actions: [
            TextButton(
              onPressed: () =>
                  l10n.setLocale(l10n.locale == 'en' ? 'zh' : 'en'),
              child: Text(
                l10n.languageToggleLabel,
                style: const TextStyle(fontSize: 14),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.wifi),
              onPressed: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const WifiManageScreen()),
                );
              },
            ),
            IconButton(
              icon: const Icon(Icons.info_outline),
              onPressed: () async {
                await Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => const DeviceInfoScreen()),
                );
              },
            ),
            IconButton(
              icon: const Icon(Icons.refresh),
              onPressed: _vm.loadDevices,
            ),
          ],
        ),
        body: _vm.loading
            ? const Center(child: CircularProgressIndicator())
            : ValueListenableBuilder<int>(
                valueListenable: _vm.statusTick,
                builder: (context, _, __) => LayoutBuilder(
                  builder: (context, constraints) {
                    final horizontalPadding = constraints.maxWidth >= 900
                        ? 24.0
                        : 0.0;
                    final maxContentWidth = constraints.maxWidth >= 1200
                        ? 1100.0
                        : (constraints.maxWidth >= 900
                              ? 960.0
                              : double.infinity);
                    return Align(
                      alignment: Alignment.topCenter,
                      child: ConstrainedBox(
                        constraints: BoxConstraints(maxWidth: maxContentWidth),
                        child: CustomScrollView(
                          slivers: [
                            SliverPadding(
                              padding: EdgeInsets.symmetric(
                                horizontal: horizontalPadding,
                              ),
                              sliver: _HomeSectionsSliver(
                                l10n: l10n,
                                devices: _vm.devices,
                                discoveredList: _vm.discoveredUnbound.values
                                    .toList(),
                                onAddDevice: _goAddDevice,
                                onOpenDevice: _openDeviceDetail,
                                onAddDiscoveredDevice: _addDiscoveredDevice,
                                resolvePresenceAndStatus: (d) =>
                                    _presenceAndStatus(l10n, d),
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                ),
              ),
        floatingActionButton: FloatingActionButton(
          onPressed: _goAddDevice,
          child: const Icon(Icons.add),
        ),
      ),
    );
  }

  Future<void> _goAddDevice() async {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const AddDeviceScreen()),
    );
    _vm.loadDevices();
  }

  static String _formatLastSeen(DateTime t) {
    final now = DateTime.now();
    if (t.year == now.year && t.month == now.month && t.day == now.day) {
      return '${t.hour.toString().padLeft(2, '0')}:${t.minute.toString().padLeft(2, '0')}';
    }
    return '${t.month.toString().padLeft(2, '0')}-${t.day.toString().padLeft(2, '0')} '
        '${t.hour.toString().padLeft(2, '0')}:${t.minute.toString().padLeft(2, '0')}';
  }
}

class _HomeSectionsSliver extends StatelessWidget {
  const _HomeSectionsSliver({
    required this.l10n,
    required this.devices,
    required this.discoveredList,
    required this.onAddDevice,
    required this.onOpenDevice,
    required this.onAddDiscoveredDevice,
    required this.resolvePresenceAndStatus,
  });

  final AppLocalizations l10n;
  final List<Device> devices;
  final List<Device> discoveredList;
  final VoidCallback onAddDevice;
  final Future<void> Function(Device device) onOpenDevice;
  final Future<void> Function(Device device) onAddDiscoveredDevice;
  final (_Presence, String) Function(Device device) resolvePresenceAndStatus;

  @override
  Widget build(BuildContext context) {
    final deviceById = <String, Device>{for (final d in devices) d.deviceId: d};

    final pairedPairs = <(Device left, Device right)>[];
    final seen = <String>{};
    for (final d in devices) {
      final peerId = d.pairedWithDeviceId;
      if (peerId == null) continue;
      final peer = deviceById[peerId];
      if (peer == null) continue;
      if (seen.contains(d.deviceId) || seen.contains(peer.deviceId)) continue;

      final left = d.deviceId.compareTo(peer.deviceId) <= 0 ? d : peer;
      final right = left.deviceId == d.deviceId ? peer : d;
      pairedPairs.add((left, right));
      seen.add(left.deviceId);
      seen.add(right.deviceId);
    }

    final unpairedBound = devices.where((d) {
      final peerId = d.pairedWithDeviceId;
      final hasValidPeer = peerId != null && deviceById.containsKey(peerId);
      return !hasValidPeer;
    }).toList();

    if (devices.isEmpty && discoveredList.isEmpty) {
      return SliverFillRemaining(
        hasScrollBody: false,
        child: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.devices_other, size: 64, color: Colors.grey),
              const SizedBox(height: 16),
              Text(l10n.t('no_bound_devices')),
              const SizedBox(height: 24),
              FilledButton.icon(
                onPressed: onAddDevice,
                icon: const Icon(Icons.add),
                label: Text(l10n.t('add_device')),
              ),
            ],
          ),
        ),
      );
    }

    return SliverList(
      delegate: SliverChildListDelegate([
        if (pairedPairs.isNotEmpty)
          _SectionHeader(text: l10n.t('paired_devices')),
        if (pairedPairs.isNotEmpty)
          _PairedSection(
            pairs: pairedPairs,
            l10n: l10n,
            resolvePresenceAndStatus: resolvePresenceAndStatus,
            onOpenDevice: onOpenDevice,
          ),
        if (unpairedBound.isNotEmpty)
          _SectionHeader(text: l10n.t('unpaired_bound_devices')),
        if (unpairedBound.isNotEmpty)
          _UnpairedBoundSection(
            devices: unpairedBound,
            l10n: l10n,
            resolvePresenceAndStatus: resolvePresenceAndStatus,
            onOpenDevice: onOpenDevice,
          ),
        if (discoveredList.isNotEmpty)
          _SectionHeader(text: l10n.t('discovered_devices')),
        if (discoveredList.isNotEmpty)
          _DiscoveredSection(
            devices: discoveredList,
            l10n: l10n,
            resolvePresenceAndStatus: resolvePresenceAndStatus,
            onAddDiscoveredDevice: onAddDiscoveredDevice,
          ),
      ]),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
      ),
    );
  }
}

class _PairedSection extends StatelessWidget {
  const _PairedSection({
    required this.pairs,
    required this.l10n,
    required this.resolvePresenceAndStatus,
    required this.onOpenDevice,
  });

  final List<(Device left, Device right)> pairs;
  final AppLocalizations l10n;
  final (_Presence, String) Function(Device device) resolvePresenceAndStatus;
  final Future<void> Function(Device device) onOpenDevice;

  _MiniDevice _buildMini(Device d) {
    final info = resolvePresenceAndStatus(d);
    final roleTag = d.isPeerShadow
        ? l10n.t('peer_device_tag')
        : l10n.t('my_device_tag');
    return _MiniDevice(
      name: d.name,
      presence: info.$1,
      statusText: info.$2,
      topRightTag: d.pairedWithDeviceId != null ? l10n.t('paired') : null,
      roleTag: roleTag,
    );
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedSize(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      child: Column(
        children: pairs.map((pair) {
          final left = pair.$1;
          final right = pair.$2;
          return Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
            child: _PairCard(
              left: left,
              right: right,
              leftMini: _buildMini(left),
              rightMini: _buildMini(right),
              onTapLeft: () => onOpenDevice(left),
              onTapRight: () => onOpenDevice(right),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _UnpairedBoundSection extends StatelessWidget {
  const _UnpairedBoundSection({
    required this.devices,
    required this.l10n,
    required this.resolvePresenceAndStatus,
    required this.onOpenDevice,
  });

  final List<Device> devices;
  final AppLocalizations l10n;
  final (_Presence, String) Function(Device device) resolvePresenceAndStatus;
  final Future<void> Function(Device device) onOpenDevice;

  @override
  Widget build(BuildContext context) {
    return AnimatedSize(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      child: _ResponsiveCardWrap(
        children: devices.map((d) {
          final info = resolvePresenceAndStatus(d);
          final presence = info.$1;
          final statusText = info.$2;
          return Card(
            color: Colors.grey.shade50,
            margin: EdgeInsets.zero,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(16),
              side: BorderSide(color: Colors.grey.shade300),
            ),
            child: InkWell(
              onTap: () => onOpenDevice(d),
              borderRadius: BorderRadius.circular(16),
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: Row(
                  children: [
                    _PresenceDot(presence: presence),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            d.name,
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                          const SizedBox(height: 6),
                          Text(
                            statusText,
                            style: Theme.of(context).textTheme.bodyMedium,
                          ),
                          if (d.lastConnectedSsid != null &&
                              d.lastConnectedSsid!.isNotEmpty)
                            Padding(
                              padding: const EdgeInsets.only(top: 6),
                              child: Text(
                                '${l10n.t('wifi')}: ${d.lastConnectedSsid}',
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                            ),
                        ],
                      ),
                    ),
                    const Icon(Icons.chevron_right),
                  ],
                ),
              ),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _DiscoveredSection extends StatelessWidget {
  const _DiscoveredSection({
    required this.devices,
    required this.l10n,
    required this.resolvePresenceAndStatus,
    required this.onAddDiscoveredDevice,
  });

  final List<Device> devices;
  final AppLocalizations l10n;
  final (_Presence, String) Function(Device device) resolvePresenceAndStatus;
  final void Function(Device device) onAddDiscoveredDevice;

  @override
  Widget build(BuildContext context) {
    return AnimatedSize(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      child: _ResponsiveCardWrap(
        children: devices.map((d) {
          final info = resolvePresenceAndStatus(d);
          final presence = info.$1;
          final statusText = info.$2;
          final ipText = d.ipAddress.isEmpty
              ? l10n.t('waiting_ip')
              : d.ipAddress;
          return Card(
            margin: EdgeInsets.zero,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(16),
            ),
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Row(
                children: [
                  _PresenceDot(presence: presence),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(
                              d.deviceId,
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                            const SizedBox(width: 8),
                            const Icon(
                              Icons.router,
                              size: 18,
                              color: Colors.grey,
                            ),
                          ],
                        ),
                        const SizedBox(height: 6),
                        Text(
                          statusText,
                          style: Theme.of(context).textTheme.bodyMedium,
                        ),
                        const SizedBox(height: 4),
                        Text(
                          ipText,
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                  FilledButton(
                    onPressed: () async => onAddDiscoveredDevice(d),
                    child: Text(l10n.t('add')),
                  ),
                ],
              ),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _ResponsiveCardWrap extends StatelessWidget {
  const _ResponsiveCardWrap({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 900 ? 2 : 1;
        const spacing = 12.0;
        final width = columns == 1
            ? constraints.maxWidth - 32
            : (constraints.maxWidth - 32 - spacing) / 2;

        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Wrap(
            spacing: spacing,
            runSpacing: spacing,
            children: children
                .map(
                  (w) => SizedBox(
                    width: width.clamp(0.0, double.infinity),
                    child: w,
                  ),
                )
                .toList(),
          ),
        );
      },
    );
  }
}

enum _Presence { online, suspected, offline }

class _PresenceDot extends StatelessWidget {
  const _PresenceDot({required this.presence});

  final _Presence presence;

  @override
  Widget build(BuildContext context) {
    Color color;
    switch (presence) {
      case _Presence.online:
        color = Colors.green;
        break;
      case _Presence.suspected:
        color = Colors.orange;
        break;
      case _Presence.offline:
        color = Colors.grey;
        break;
    }

    return AnimatedContainer(
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeOutCubic,
      width: 12,
      height: 12,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
        boxShadow: [
          BoxShadow(
            blurRadius: 6,
            color: color.withAlpha((0.35 * 255).round()),
            spreadRadius: 1,
          ),
        ],
      ),
    );
  }
}

class _MiniDevice extends StatelessWidget {
  const _MiniDevice({
    required this.name,
    required this.presence,
    required this.statusText,
    required this.roleTag,
    this.topRightTag,
  });

  final String name;
  final _Presence presence;
  final String statusText;
  final String roleTag;
  final String? topRightTag;

  @override
  Widget build(BuildContext context) {
    final dot = _PresenceDot(presence: presence);
    final borderColor = presence == _Presence.online
        ? Colors.green.withAlpha((0.35 * 255).round())
        : (presence == _Presence.suspected
              ? Colors.orange.withAlpha((0.35 * 255).round())
              : Colors.grey.withAlpha((0.35 * 255).round()));

    return AnimatedContainer(
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeOutCubic,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: borderColor),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              dot,
              const SizedBox(width: 10),
              const Icon(Icons.devices, size: 22, color: Colors.indigo),
              const Spacer(),
              if (topRightTag != null)
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.indigo.withAlpha((0.08 * 255).round()),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    topRightTag!,
                    style: Theme.of(
                      context,
                    ).textTheme.labelSmall?.copyWith(color: Colors.indigo),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 10),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: Colors.grey.withAlpha((0.12 * 255).round()),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Text(
              roleTag,
              style: Theme.of(context).textTheme.labelSmall,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 6),
          Text(
            statusText,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}

class _PairCard extends StatelessWidget {
  const _PairCard({
    required this.left,
    required this.right,
    required this.leftMini,
    required this.rightMini,
    required this.onTapLeft,
    required this.onTapRight,
  });

  final Device left;
  final Device right;
  final _MiniDevice leftMini;
  final _MiniDevice rightMini;
  final Future<void> Function() onTapLeft;
  final Future<void> Function() onTapRight;

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Colors.white,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 620;
            if (compact) {
              return Column(
                children: [
                  InkWell(
                    onTap: onTapLeft,
                    borderRadius: BorderRadius.circular(16),
                    child: leftMini,
                  ),
                  const SizedBox(height: 10),
                  _LinkChain(compact: true),
                  const SizedBox(height: 10),
                  InkWell(
                    onTap: onTapRight,
                    borderRadius: BorderRadius.circular(16),
                    child: rightMini,
                  ),
                ],
              );
            }
            return Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                Expanded(
                  child: InkWell(
                    onTap: onTapLeft,
                    borderRadius: BorderRadius.circular(16),
                    child: leftMini,
                  ),
                ),
                const SizedBox(width: 10),
                _LinkChain(compact: false),
                const SizedBox(width: 10),
                Expanded(
                  child: InkWell(
                    onTap: onTapRight,
                    borderRadius: BorderRadius.circular(16),
                    child: rightMini,
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _LinkChain extends StatelessWidget {
  const _LinkChain({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Stack(
          alignment: Alignment.center,
          children: [
            Icon(
              Icons.link,
              size: compact ? 30 : 38,
              color: Colors.indigo,
            ),
            Positioned(
              bottom: 2,
              child: Container(
                padding: const EdgeInsets.all(4),
                decoration: BoxDecoration(
                  color: Colors.indigo.withAlpha((0.10 * 255).round()),
                  shape: BoxShape.circle,
                ),
                child: const Icon(Icons.lock, size: 16, color: Colors.indigo),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
