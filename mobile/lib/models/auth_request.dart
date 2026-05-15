enum OperationType {
  fileWrite,
  fileDelete,
  shellExecute,
  gitPush,
  packageInstall,
  configModify,
  unknown,
}

enum RiskLevel {
  low,
  medium,
  high,
  critical,
  unknown,
}

enum Decision {
  approved,
  rejected,
  timeout,
}

class FileChange {
  final String path;
  final String changeType;
  final String diff;
  final int additions;
  final int deletions;

  FileChange({
    required this.path,
    required this.changeType,
    required this.diff,
    required this.additions,
    required this.deletions,
  });

  factory FileChange.fromMap(Map<String, dynamic> map) {
    return FileChange(
      path: map['path'] ?? '',
      changeType: map['changeType'] ?? 'unknown',
      diff: map['diff'] ?? '',
      additions: map['additions'] ?? 0,
      deletions: map['deletions'] ?? 0,
    );
  }
}

class AuthRequest {
  final String requestId;
  final String toolName;
  final OperationType operation;
  final RiskLevel riskLevel;
  final String summary;
  final List<FileChange> files;
  final String rawCommand;
  final int timeoutSeconds;
  final DateTime receivedAt;

  AuthRequest({
    required this.requestId,
    required this.toolName,
    required this.operation,
    required this.riskLevel,
    required this.summary,
    required this.files,
    required this.rawCommand,
    required this.timeoutSeconds,
    DateTime? receivedAt,
  }) : receivedAt = receivedAt ?? DateTime.now();

  String get toolDisplayName {
    switch (toolName) {
      case 'claude_code':
        return 'Claude Code';
      case 'aider':
        return 'Aider';
      case 'codex':
        return 'Codex';
      default:
        return toolName;
    }
  }

  String get operationDisplayName {
    switch (operation) {
      case OperationType.fileWrite:
        return '写入文件';
      case OperationType.fileDelete:
        return '删除文件';
      case OperationType.shellExecute:
        return '执行命令';
      case OperationType.gitPush:
        return 'Git Push';
      case OperationType.packageInstall:
        return '安装包';
      case OperationType.configModify:
        return '修改配置';
      default:
        return '未知操作';
    }
  }

  String get riskDisplayName {
    switch (riskLevel) {
      case RiskLevel.low:
        return 'LOW';
      case RiskLevel.medium:
        return 'MEDIUM';
      case RiskLevel.high:
        return 'HIGH';
      case RiskLevel.critical:
        return 'CRITICAL';
      default:
        return 'UNKNOWN';
    }
  }
}
