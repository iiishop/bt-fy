import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import '../constants/protocol.dart';
import '../models/wifi_network.dart';

/// 配网阶段：连接设备热点后，通过 TCP 发送 identify / config（设计 5.1）
class DeviceProvisioningService {
  /// 统一提取错误信息：优先 message，回退 reason，再回退 code。
  static String? pickErrorMessage(Map<String, dynamic>? payload) {
    if (payload == null) return null;
    final message = payload['message']?.toString().trim();
    if (message != null && message.isNotEmpty) return message;
    final reason = payload['reason']?.toString().trim();
    if (reason != null && reason.isNotEmpty) return reason;
    final code = payload['code']?.toString().trim();
    if (code != null && code.isNotEmpty) return code;
    return null;
  }

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

  /// 发送已保存的全部 Wi-Fi 列表；设备按信号强度选最佳连接。发完即关闭，手机可切回原 WiFi，设备连上后发 binding 完成绑定。
  Future<Map<String, dynamic>> config({
    required List<WifiNetwork> networks,
    String? bindToken,
    String host = Protocol.deviceApGateway,
    int port = Protocol.apTcpPort,
  }) async {
    if (networks.isEmpty) {
      return {
        'status': 'error',
        'code': 'no_networks',
        'message': 'No networks',
        'reason': 'No networks',
      };
    }
    try {
      final socket = await Socket.connect(host, port, timeout: const Duration(seconds: 8));
      try {
        final list = networks.map((w) => {
          'ssid': w.ssid,
          'pwd': w.password,
          'sec': w.securityType,
        }).toList();
        final body = <String, dynamic>{
          'cmd': 'config',
          'networks': list,
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
        if (lines.isEmpty) {
          return {
            'status': 'error',
            'code': 'empty_response',
            'message': '设备未返回数据',
            'reason': '设备未返回数据',
          };
        }
        return jsonDecode(lines.first) as Map<String, dynamic>? ?? {};
      } finally {
        socket.destroy();
      }
    } on SocketException catch (e) {
      final msg = e.message;
      return {
        'status': 'error',
        'code': 'socket_error',
        'message': msg,
        'reason': msg,
      };
    } on Exception catch (e) {
      final msg = e.toString();
      return {
        'status': 'error',
        'code': 'config_exception',
        'message': msg,
        'reason': msg,
      };
    }
  }
}
