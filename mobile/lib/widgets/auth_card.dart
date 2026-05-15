import 'package:flutter/material.dart';
import '../models/auth_request.dart';

class AuthCard extends StatelessWidget {
  final AuthRequest request;
  final VoidCallback onApprove;
  final VoidCallback onReject;
  final VoidCallback? onTap;

  const AuthCard({
    super.key,
    required this.request,
    required this.onApprove,
    required this.onReject,
    this.onTap,
  });

  Color get _riskColor {
    switch (request.riskLevel) {
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

  @override
  Widget build(BuildContext context) {
    return Card(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: const Color(0xFF0F0F0F),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      request.toolDisplayName,
                      style: const TextStyle(fontSize: 12),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: _riskColor,
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      request.riskDisplayName,
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const Spacer(),
                  Text(
                    request.operationDisplayName,
                    style: TextStyle(fontSize: 12, color: Colors.grey[500]),
                  ),
                ],
              ),

              const SizedBox(height: 12),
              Text(
                request.summary,
                style: const TextStyle(fontSize: 14, height: 1.4),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),

              const SizedBox(height: 8),
              Text(
                '${request.files.length} 个文件 · 超时 ${request.timeoutSeconds}s',
                style: TextStyle(fontSize: 12, color: Colors.grey[500]),
              ),

              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: SizedBox(
                      height: 40,
                      child: OutlinedButton(
                        onPressed: onReject,
                        style: OutlinedButton.styleFrom(
                          foregroundColor: Colors.red,
                          side: const BorderSide(color: Colors.red),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ),
                        child: const Text('拒绝'),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: SizedBox(
                      height: 40,
                      child: FilledButton(
                        onPressed: onApprove,
                        style: FilledButton.styleFrom(
                          backgroundColor: Colors.green,
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ),
                        child: const Text('允许'),
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
