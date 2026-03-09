import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import '../constants/protocol.dart';
import '../models/device.dart';

/// STA 模式下：监听 UDP 广播（hello/heartbeat/binding），并通过 TCP 控制设备（设计 5.2）
class DeviceDiscoveryService {
  RawDatagramSocket? _udpSocket;
  StreamSubscription<RawSocketEvent>? _sub;
  final void Function(Device device)? onDeviceSeen;
  /// evt=binding 时回调 (deviceId, ip, bindToken)，用于手机回信完成绑定
  final void Function(String deviceId, String ip, String bindToken)? onBindingSeen;

  DeviceDiscoveryService({this.onDeviceSeen, this.onBindingSeen});

  /// 开始监听 UDP 广播（端口 12345）
  Future<bool> startListening() async {
    if (_udpSocket != null) return true;
    try {
      _udpSocket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, Protocol.staUdpPort);
      _udpSocket!.broadcastEnabled = true;
      _sub = _udpSocket!.listen((event) {
        if (event != RawSocketEvent.read) return;
        final dgram = _udpSocket!.receive();
        if (dgram == null) return;
        _handleBroadcast(dgram.data);
      });
      return true;
    } on SocketException catch (_) {
      return false;
    }
  }

  void _handleBroadcast(List<int> data) {
    try {
      final s = utf8.decode(data);
      final map = jsonDecode(s) as Map<String, dynamic>?;
      if (map == null) return;
      final evt = map['evt'] as String?;
      final id = map['id'] as String?;
      if (id == null) return;
      final ip = map['ip'] as String? ?? '';
      if (evt == 'binding') {
        final bindToken = map['bindToken'] as String?;
        if (bindToken != null && bindToken.isNotEmpty) {
          onBindingSeen?.call(id, ip, bindToken);
        }
      }
      final device = Device(
        deviceId: id,
        name: id,
        ipAddress: ip,
        isOnline: true,
        lastSeen: DateTime.now(),
        isBound: evt == 'heartbeat',
      );
      onDeviceSeen?.call(device);
    } catch (_) {}
  }

  void stopListening() {
    _sub?.cancel();
    _sub = null;
    _udpSocket?.close();
    _udpSocket = null;
  }

  /// 通过 TCP 发送控制指令（设计 5.2）
  static Future<Map<String, dynamic>> sendCommand(
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
}
