import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import '../constants/protocol.dart';
import '../models/device.dart';

/// STA 模式下：监听 UDP 广播（hello/heartbeat/binding），并通过 TCP 控制设备（设计 5.2）
class DeviceDiscoveryService {
  static RawDatagramSocket? _sharedUdpSocket;
  static StreamSubscription<RawSocketEvent>? _sharedSub;
  static final Map<int, DeviceDiscoveryService> _listeners = {};
  static int _nextListenerId = 1;
  static final Map<String, Future<Map<String, dynamic>>> _inflightReadOnly = {};

  final int _listenerId = _nextListenerId++;
  bool _subscribed = false;
  final void Function(Device device)? onDeviceSeen;
  /// evt=binding 时回调 (deviceId, ip, bindToken)，用于手机回信完成绑定
  final void Function(String deviceId, String ip, String bindToken)? onBindingSeen;

  DeviceDiscoveryService({this.onDeviceSeen, this.onBindingSeen});

  /// 开始监听 UDP 广播（端口 12345）
  Future<bool> startListening() async {
    if (_subscribed) return true;
    try {
      if (_sharedUdpSocket == null) {
        _sharedUdpSocket = await RawDatagramSocket.bind(
          InternetAddress.anyIPv4,
          Protocol.staUdpPort,
        );
        _sharedUdpSocket!.broadcastEnabled = true;
        _sharedSub = _sharedUdpSocket!.listen((event) {
          if (event != RawSocketEvent.read) return;
          final dgram = _sharedUdpSocket!.receive();
          if (dgram == null) return;
          final fromIp = dgram.address.address;
          final snapshot = List<DeviceDiscoveryService>.from(_listeners.values);
          for (final listener in snapshot) {
            listener._handleBroadcast(dgram.data, fromIp);
          }
        });
      }
      _listeners[_listenerId] = this;
      _subscribed = true;
      return true;
    } on SocketException catch (_) {
      return false;
    }
  }

  void _handleBroadcast(List<int> data, String fromIp) {
    try {
      final s = utf8.decode(data);
      final map = jsonDecode(s) as Map<String, dynamic>?;
      if (map == null) return;
      final evt = map['evt'] as String?;
      final id = map['id'] as String?;
      if (id == null) return;
      final ip = map['ip'] as String? ?? '';
      final effectiveIp = ip.isNotEmpty && ip != '0.0.0.0' ? ip : fromIp;
      final ssid = map['ssid'] as String?;
      final lastSsid = (ssid != null && ssid.isNotEmpty) ? ssid : null;
      if (evt == 'binding') {
        final bindToken = map['bindToken'] as String?;
        if (bindToken != null && bindToken.isNotEmpty) {
          onBindingSeen?.call(id, effectiveIp, bindToken);
        }
      }
      final device = Device(
        deviceId: id,
        name: id,
        ipAddress: effectiveIp,
        isOnline: true,
        lastSeen: DateTime.now(),
        isBound: evt == 'heartbeat',
        lastConnectedSsid: lastSsid,
      );
      onDeviceSeen?.call(device);
    } catch (_) {}
  }

  void stopListening() {
    if (!_subscribed) return;
    _listeners.remove(_listenerId);
    _subscribed = false;
    if (_listeners.isNotEmpty) return;
    _sharedSub?.cancel();
    _sharedSub = null;
    _sharedUdpSocket?.close();
    _sharedUdpSocket = null;
  }

  /// 通过 TCP 发送控制指令（设计 5.2）
  static Future<Map<String, dynamic>> sendCommand(
    String host,
    int port,
    Map<String, dynamic> command,
  ) async {
    final cmd = command['cmd'] as String? ?? '';
    final readOnly = cmd == 'get_pair_status' || cmd == 'get_pending_pair_requests';
    if (!readOnly) {
      return _sendCommandInternal(host, port, command);
    }
    final key = '$host:$port:$cmd';
    final existing = _inflightReadOnly[key];
    if (existing != null) return existing;
    final future = _sendCommandInternal(host, port, command).whenComplete(() {
      _inflightReadOnly.remove(key);
    });
    _inflightReadOnly[key] = future;
    return future;
  }

  static Future<Map<String, dynamic>> _sendCommandInternal(
    String host,
    int port,
    Map<String, dynamic> command,
  ) async {
    try {
      final socket = await Socket.connect(host, port, timeout: const Duration(seconds: 5));
      try {
        socket.write('${jsonEncode(command)}\n');
        await socket.flush();
        final line = await socket
            .transform(StreamTransformer<Uint8List, String>.fromHandlers(handleData: (data, sink) => sink.add(utf8.decode(data))))
            .transform(const LineSplitter())
            .first
            .timeout(const Duration(seconds: 10));
        socket.destroy();
        return jsonDecode(line) as Map<String, dynamic>? ?? {};
      } finally {
        socket.destroy();
      }
    } on Exception catch (e) {
      return {'status': 'error', 'reason': e.toString()};
    }
  }

  /// 演示舵机 40°→120°→40°
  static Future<Map<String, dynamic>> demoServo(String host, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'demo_servo'});
  }

  /// 单舵机角度
  static Future<Map<String, dynamic>> moveServo(String host, int servoIndex, int angle, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'move_servo', 'servo': servoIndex, 'angle': angle});
  }

  /// 绑定（phone 为手机唯一标识）
  static Future<Map<String, dynamic>> bind(String host, String phoneId, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'bind', 'phone': phoneId});
  }

  /// 解绑
  static Future<Map<String, dynamic>> unbind(String host, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'unbind'});
  }

  /// 发起配对请求（本机 ESP 会向 targetIp 的 ESP 发送 pair_request）
  static Future<Map<String, dynamic>> pairRequest(
    String myDeviceHost,
    String targetDeviceId,
    String targetIp, {
    int port = Protocol.staTcpPort,
  }) {
    return sendCommand(myDeviceHost, port, {
      'cmd': 'pair_request',
      'target_ip': targetIp,
      'target_device_id': targetDeviceId,
    });
  }

  /// 获取待处理的配对请求（来自其他 ESP）
  static Future<Map<String, dynamic>> getPendingPairRequests(String host, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'get_pending_pair_requests'});
  }

  /// 接受配对（向对方 ESP 发送 pair_accepted）
  static Future<Map<String, dynamic>> acceptPair(String host, String fromDeviceId, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'accept_pair', 'from_device_id': fromDeviceId});
  }

  /// 拒绝配对（从 pending 列表移除）
  static Future<Map<String, dynamic>> rejectPair(String host, String fromDeviceId, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'reject_pair', 'from_device_id': fromDeviceId});
  }

  /// 查询本设备当前配对对象（用于 A 端轮询是否已被 B 接受）
  static Future<Map<String, dynamic>> getPairStatus(String host, {int port = Protocol.staTcpPort}) {
    return sendCommand(host, port, {'cmd': 'get_pair_status'});
  }

  /// 解除配对（并可选通知对方 peer_ip）
  static Future<Map<String, dynamic>> unpair(String host, {String? peerIp, int port = Protocol.staTcpPort}) {
    final cmd = <String, dynamic>{'cmd': 'unpair'};
    if (peerIp != null && peerIp.isNotEmpty) cmd['peer_ip'] = peerIp;
    return sendCommand(host, port, cmd);
  }

  /// 同步 Flutter 中保存的 WiFi 列表到设备（每次与设备通讯时调用，使设备端表与 App 一致）
  static Future<Map<String, dynamic>> updateWifiList(
    String host,
    List<Map<String, dynamic>> networks, {
    int port = Protocol.staTcpPort,
  }) {
    return sendCommand(host, port, {'cmd': 'update_wifi_list', 'networks': networks});
  }

  // 非同一局域网配对预留：未来可通过 P2P / Tailscale 等解析 target_device_id 得到可达地址后再调用 pairRequest。
  // static Future<Map<String, dynamic>> pairRequestRemote(String myDeviceHost, String targetDeviceId) => ...
}
