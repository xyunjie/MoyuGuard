import 'dart:async';
import 'dart:convert';
import 'dart:io' show Platform;
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import '../models/auth_request.dart';

enum PairState { idle, waiting, rejected }

const _liveActivityChannel = MethodChannel('moyuguard/live_activity');

bool get _supportsLiveActivity =>
    !kIsWeb && Platform.isIOS;

class ConnectionService extends ChangeNotifier {
  WebSocketChannel? _channel;
  String? _serverAddress;
  String? _computerName;
  bool _isConnected = false;
  PairState _pairState = PairState.idle;
  final List<AuthRequest> _pendingRequests = [];
  final List<String> _log = [];
  Timer? _heartbeatTimer;
  String? _deviceId;

  bool get isConnected => _isConnected;
  PairState get pairState => _pairState;
  String? get serverAddress => _serverAddress;
  String? get computerName => _computerName;
  List<AuthRequest> get pendingRequests => List.unmodifiable(_pendingRequests);
  List<String> get log => List.unmodifiable(_log);

  Future<String> _getDeviceId() async {
    if (_deviceId != null) return _deviceId!;
    final prefs = await SharedPreferences.getInstance();
    var id = prefs.getString('device_id');
    if (id == null) {
      id = 'flutter-${DateTime.now().millisecondsSinceEpoch}';
      await prefs.setString('device_id', id);
    }
    _deviceId = id;
    return id;
  }

  Future<void> connect(String host, int port) async {
    try {
      final uri = Uri.parse('ws://$host:$port');
      _channel = WebSocketChannel.connect(uri);
      await _channel!.ready;

      _serverAddress = '$host:$port';
      _isConnected = true;
      _pairState = PairState.waiting;
      notifyListeners();

      _startHeartbeat();
      _listenMessages();
      _setupLiveActivityActionHandler();

      final deviceId = await _getDeviceId();
      _channel?.sink.add(jsonEncode({
        'type': 'pair_request',
        'device_id': deviceId,
        'device_name': kIsWeb ? 'Flutter Web' : 'Flutter Mobile',
        'platform': kIsWeb ? 'web' : 'mobile',
      }));

      _addLog('已连接到 $host:$port，等待配对确认…');
    } catch (e) {
      _addLog('连接失败: $e');
      _isConnected = false;
      _pairState = PairState.idle;
      notifyListeners();
    }
  }

  void _listenMessages() {
    _channel?.stream.listen(
      (data) {
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
        if (_pairState != PairState.idle) return; // only trusted clients reach here
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
        _updateLiveActivity();
      } else if (type == 'pair_response') {
        final accepted = msg['accepted'] as bool? ?? false;
        if (accepted) {
          _computerName = msg['computer_name'];
          _pairState = PairState.idle;
          _addLog('配对成功: $_computerName');
        } else {
          _pairState = PairState.rejected;
          _addLog('配对被拒绝');
        }
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
    _updateLiveActivity();
  }

  void _setupLiveActivityActionHandler() {
    if (!_supportsLiveActivity) return;
    _liveActivityChannel.setMethodCallHandler((call) async {
      if (call.method == 'handleAction') {
        final args = call.arguments as Map;
        final action = args['action'] as String;
        final requestId = args['requestId'] as String;
        final decision = action == 'approve' ? Decision.approved : Decision.rejected;
        sendDecision(requestId, decision);
      }
    });
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
    _pairState = PairState.idle;
    _serverAddress = null;
    _computerName = null;
    _pendingRequests.clear();
    notifyListeners();
    if (_supportsLiveActivity) {
      _liveActivityChannel.invokeMethod('end').catchError((_) {});
    }
  }

  void _updateLiveActivity() {
    if (!_supportsLiveActivity) return;
    debugPrint('[LiveActivity] updating, pending=${_pendingRequests.length}');

    // Keep activity alive even when pending=0 so background updates work.
    // Only end() when disconnecting (called in _disconnect).
    final args = _pendingRequests.isEmpty
        ? {'pendingCount': 0, 'summary': '一切安全，放心摸鱼', 'risk': 'low', 'requestId': ''}
        : {
            'pendingCount': _pendingRequests.length,
            'summary': _pendingRequests.last.summary,
            'risk': _pendingRequests.last.riskLevel.name,
            'requestId': _pendingRequests.last.requestId,
          };

    _liveActivityChannel.invokeMethod('update', args).then((_) {
      debugPrint('[LiveActivity] update OK');
    }).catchError((e) {
      final msg = e.toString();
      if (msg.contains('visibility')) {
        // ActivityKit: cannot START from background — normal behavior.
        debugPrint('[LiveActivity] cannot start from background (OK)');
        return;
      }
      debugPrint('[LiveActivity] update/start error: $e');
      _liveActivityChannel.invokeMethod('start', args).then((_) {
        debugPrint('[LiveActivity] start OK');
      }).catchError((e2) {
        if (!e2.toString().contains('visibility')) {
          debugPrint('[LiveActivity] start error: $e2');
        }
      });
    });
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
