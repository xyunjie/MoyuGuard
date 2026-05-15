import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import '../models/auth_request.dart';

class ConnectionService extends ChangeNotifier {
  WebSocketChannel? _channel;
  String? _serverAddress;
  String? _computerName;
  bool _isConnected = false;
  final List<AuthRequest> _pendingRequests = [];
  final List<String> _log = [];
  Timer? _heartbeatTimer;

  bool get isConnected => _isConnected;
  String? get serverAddress => _serverAddress;
  String? get computerName => _computerName;
  List<AuthRequest> get pendingRequests => List.unmodifiable(_pendingRequests);
  List<String> get log => List.unmodifiable(_log);

  Future<void> connect(String host, int port) async {
    try {
      final uri = Uri.parse('ws://$host:$port');
      _channel = WebSocketChannel.connect(uri);
      await _channel!.ready;

      _serverAddress = '$host:$port';
      _isConnected = true;
      notifyListeners();

      _startHeartbeat();
      _listenMessages();

      _addLog('已连接到 $host:$port');
    } catch (e) {
      _addLog('连接失败: $e');
      _isConnected = false;
      notifyListeners();
    }
  }

  void _listenMessages() {
    _channel?.stream.listen(
      (data) {
        // MVP: 使用 JSON 简化调试，后续切换为 Protobuf
        if (data is String) {
          _handleJsonMessage(data);
        }
      },
      onDone: () {
        _addLog('连接断开');
        _disconnect();
      },
      onError: (error) {
        _addLog('连接错误: $error');
        _disconnect();
      },
    );
  }

  void _handleJsonMessage(String data) {
    try {
      final msg = jsonDecode(data) as Map<String, dynamic>;
      final type = msg['type'] as String?;

      if (type == 'auth_request') {
        final req = AuthRequest(
          requestId: msg['request_id'] ?? '',
          toolName: msg['tool_name'] ?? 'unknown',
          operation: _parseOperation(msg['operation'] ?? ''),
          riskLevel: _parseRiskLevel(msg['risk_level'] ?? ''),
          summary: msg['summary'] ?? '',
          files: (msg['files'] as List?)
                  ?.map((f) => FileChange.fromMap(f as Map<String, dynamic>))
                  .toList() ??
              [],
          rawCommand: msg['raw_command'] ?? '',
          timeoutSeconds: msg['timeout_seconds'] ?? 60,
        );
        _pendingRequests.add(req);
        _addLog('收到授权请求: ${req.toolDisplayName} - ${req.summary}');
        notifyListeners();
      } else if (type == 'pair_response') {
        _computerName = msg['computer_name'];
        _addLog('配对成功: $_computerName');
        notifyListeners();
      }
    } catch (e) {
      _addLog('消息解析失败: $e');
    }
  }

  void sendDecision(String requestId, Decision decision, {String reason = ''}) {
    final msg = jsonEncode({
      'type': 'auth_response',
      'request_id': requestId,
      'decision': decision == Decision.approved ? 'approved' : 'rejected',
      'reason': reason,
    });

    _channel?.sink.add(msg);
    _pendingRequests.removeWhere((r) => r.requestId == requestId);
    _addLog(
      '${decision == Decision.approved ? "✅ 已批准" : "❌ 已拒绝"}: $requestId',
    );
    notifyListeners();
  }

  void _startHeartbeat() {
    _heartbeatTimer?.cancel();
    _heartbeatTimer = Timer.periodic(const Duration(seconds: 30), (_) {
      _channel?.sink.add(jsonEncode({'type': 'heartbeat'}));
    });
  }

  void _disconnect() {
    _heartbeatTimer?.cancel();
    _channel?.sink.close();
    _channel = null;
    _isConnected = false;
    _serverAddress = null;
    _computerName = null;
    _pendingRequests.clear();
    notifyListeners();
  }

  void disconnect() {
    _addLog('主动断开连接');
    _disconnect();
  }

  void _addLog(String message) {
    final time = DateTime.now().toString().substring(11, 19);
    _log.insert(0, '[$time] $message');
    if (_log.length > 100) _log.removeLast();
  }

  OperationType _parseOperation(String op) {
    switch (op) {
      case 'file_write':
        return OperationType.fileWrite;
      case 'file_delete':
        return OperationType.fileDelete;
      case 'shell_execute':
        return OperationType.shellExecute;
      case 'git_push':
        return OperationType.gitPush;
      case 'package_install':
        return OperationType.packageInstall;
      case 'config_modify':
        return OperationType.configModify;
      default:
        return OperationType.unknown;
    }
  }

  RiskLevel _parseRiskLevel(String level) {
    switch (level) {
      case 'low':
        return RiskLevel.low;
      case 'medium':
        return RiskLevel.medium;
      case 'high':
        return RiskLevel.high;
      case 'critical':
        return RiskLevel.critical;
      default:
        return RiskLevel.unknown;
    }
  }

  @override
  void dispose() {
    _disconnect();
    super.dispose();
  }
}
