import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/auth_request.dart';
import '../services/connection_service.dart';

class AuthScreen extends StatelessWidget {
  final AuthRequest request;

  const AuthScreen({super.key, required this.request});

  @override
  Widget build(BuildContext context) {
    final riskColor = _riskColor(request.riskLevel);

    return Scaffold(
      appBar: AppBar(
        title: const Text('授权详情'),
        backgroundColor: const Color(0xFF1A1A1A),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: const Color(0xFF0F0F0F),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    request.toolDisplayName,
                    style: const TextStyle(fontWeight: FontWeight.w500),
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: riskColor,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    request.riskDisplayName,
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                      fontSize: 12,
                    ),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 20),
            Text(
              request.summary,
              style: const TextStyle(fontSize: 16, height: 1.5),
            ),

            const SizedBox(height: 16),
            _infoRow('操作类型', request.operationDisplayName),
            _infoRow('涉及文件', '${request.files.length} 个'),
            _infoRow('超时时间', '${request.timeoutSeconds} 秒'),
            if (request.rawCommand.isNotEmpty)
              _infoRow('原始命令', request.rawCommand),

            if (request.files.isNotEmpty) ...[
              const SizedBox(height: 24),
              const Text(
                '文件变更',
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  color: Colors.grey,
                ),
              ),
              const SizedBox(height: 12),
              ...request.files.map(
                (file) => Container(
                  width: double.infinity,
                  margin: const EdgeInsets.only(bottom: 8),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: const Color(0xFF1A1A1A),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: const Color(0xFF333333)),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        file.path,
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Row(
                        children: [
                          Text(
                            '+${file.additions}',
                            style: const TextStyle(
                              color: Colors.green,
                              fontSize: 12,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Text(
                            '-${file.deletions}',
                            style: const TextStyle(
                              color: Colors.red,
                              fontSize: 12,
                            ),
                          ),
                        ],
                      ),
                      if (file.diff.isNotEmpty) ...[
                        const SizedBox(height: 8),
                        Container(
                          width: double.infinity,
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: const Color(0xFF0F0F0F),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Text(
                            file.diff,
                            style: const TextStyle(
                              fontFamily: 'monospace',
                              fontSize: 12,
                              height: 1.4,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
            ],

            const SizedBox(height: 32),
            Row(
              children: [
                Expanded(
                  child: SizedBox(
                    height: 52,
                    child: OutlinedButton(
                      onPressed: () {
                        context.read<ConnectionService>().sendDecision(
                              request.requestId,
                              Decision.rejected,
                            );
                        Navigator.of(context).pop();
                      },
                      style: OutlinedButton.styleFrom(
                        foregroundColor: Colors.red,
                        side: const BorderSide(color: Colors.red),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12),
                        ),
                      ),
                      child: const Text(
                        '拒绝',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: SizedBox(
                    height: 52,
                    child: FilledButton(
                      onPressed: () {
                        context.read<ConnectionService>().sendDecision(
                              request.requestId,
                              Decision.approved,
                            );
                        Navigator.of(context).pop();
                      },
                      style: FilledButton.styleFrom(
                        backgroundColor: Colors.green,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12),
                        ),
                      ),
                      child: const Text(
                        '允许',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _infoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 80,
            child: Text(
              label,
              style: TextStyle(fontSize: 13, color: Colors.grey[500]),
            ),
          ),
          Expanded(
            child: Text(value, style: const TextStyle(fontSize: 13)),
          ),
        ],
      ),
    );
  }

  Color _riskColor(RiskLevel level) {
    switch (level) {
      case RiskLevel.low:
        return Colors.green;
      case RiskLevel.medium:
        return Colors.amber;
      case RiskLevel.high:
        return Colors.orange;
      case RiskLevel.critical:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }
}
