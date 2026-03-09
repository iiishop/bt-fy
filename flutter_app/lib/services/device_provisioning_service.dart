import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import '../constants/protocol.dart';
import '../models/wifi_network.dart';

/// 配网阶段：连接设备热点后，通过 TCP 发送 identify / config（设计 5.1）
class DeviceProvisioningService {
  /// 请求设备身份（AP 模式下，设备 IP 为 deviceApGateway）
  /// 返回 null 表示失败；若需错误信息可传 [onError]。
  Future<Map<String, dynamic>?> identify({
    String host = Protocol.deviceApGateway,
    int port = Protocol.apTcpPort,
    void Function(String message)? onError,
  }) async {
    try {
      final socket = await Socket.connect(host, port, timeout: const Duration(seconds: 10));
      try {
        socket.write('{"cmd":"identify"}\n');
        await socket.flush();
        // 使用 take(1).toList() 避免流结束无数据时 .first 抛 Bad state: No element
        final lines = await socket
            .transform(StreamTransformer<Uint8List, String>.fromHandlers(handleData: (data, sink) => sink.add(utf8.decode(data))))
            .transform(const LineSplitter())
            .where((s) => s.isNotEmpty)
            .take(1)
            .toList()
            .timeout(const Duration(seconds: 8));
        socket.destroy();
        if (lines.isEmpty) {
          onError?.call('设备未返回数据');
          return null;
        }
        final map = jsonDecode(lines.first) as Map<String, dynamic>?;
        return map;
      } finally {
        socket.destroy();
      }
    } on SocketException catch (e) {
      onError?.call('网络异常: ${e.message}');
      return null;
    } on TimeoutException catch (_) {
      onError?.call('等待设备响应超时');
      return null;
    } on Exception catch (e) {
      onError?.call('识别失败: $e');
      return null;
    }
  }

  /// 发送 Wi-Fi 配置；发完即关闭，不等待 STA 连上。手机可立即切回原 WiFi，设备连上后会持续发 binding，手机监听到后回信完成绑定。
  Future<Map<String, dynamic>> config({
    required WifiNetwork wifi,
    String? bindToken,
    String host = Protocol.deviceApGateway,
    int port = Protocol.apTcpPort,
  }) async {
    try {
      final socket = await Socket.connect(host, port, timeout: const Duration(seconds: 8));
      try {
        final sec = wifi.securityType;
        final body = <String, dynamic>{
          'cmd': 'config',
          'ssid': wifi.ssid,
          'pwd': wifi.password,
          'sec': sec,
        };
        if (bindToken != null && bindToken.isNotEmpty) body['phone'] = bindToken;
        socket.write('${jsonEncode(body)}\n');
        await socket.flush();
        // 只等一行 "connecting" 即可，随后手机可离开热点
        final lines = await socket
            .transform(StreamTransformer<Uint8List, String>.fromHandlers(handleData: (data, sink) => sink.add(utf8.decode(data))))
            .transform(const LineSplitter())
            .where((s) => s.isNotEmpty)
            .take(1)
            .toList()
            .timeout(const Duration(seconds: 15));
        if (lines.isEmpty) return {'status': 'error', 'reason': '设备未返回数据'};
        return jsonDecode(lines.first) as Map<String, dynamic>? ?? {};
      } finally {
        socket.destroy();
      }
    } on SocketException catch (e) {
      return {'status': 'error', 'reason': e.message};
    } on Exception catch (e) {
      return {'status': 'error', 'reason': e.toString()};
    }
  }
}
