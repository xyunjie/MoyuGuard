import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/auth_request.dart';
import '../services/connection_service.dart';
import '../widgets/auth_card.dart';
import 'auth_screen.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<ConnectionService>(
      builder: (context, service, _) {
        return Scaffold(
          appBar: AppBar(
            title: const Text('🐟 摸鱼守卫'),
            backgroundColor: const Color(0xFF1A1A1A),
            actions: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: service.isConnected ? Colors.green : Colors.grey,
                      shape: BoxShape.circle,
                      boxShadow: service.isConnected
                          ? [
                              BoxShadow(
                                color: Colors.green.withValues(alpha: 0.5),
                                blurRadius: 6,
                              ),
                            ]
                          : null,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    service.computerName ?? service.serverAddress ?? '',
                    style: TextStyle(fontSize: 12, color: Colors.grey[400]),
                  ),
                ],
              ),
              IconButton(
                icon: const Icon(Icons.link_off),
                onPressed: service.disconnect,
                tooltip: '断开连接',
              ),
            ],
          ),
          body: service.pendingRequests.isEmpty
              ? _buildEmptyState()
              : _buildRequestList(context, service),
        );
      },
    );
  }

  Widget _buildEmptyState() {
    return const Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.security, size: 64, color: Colors.grey),
          SizedBox(height: 16),
          Text(
            '一切安全，放心摸鱼',
            style: TextStyle(fontSize: 16, color: Colors.grey),
          ),
          SizedBox(height: 8),
          Text(
            'AI 工具执行危险操作时会在此显示',
            style: TextStyle(fontSize: 12, color: Colors.grey),
          ),
        ],
      ),
    );
  }

  Widget _buildRequestList(BuildContext context, ConnectionService service) {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: service.pendingRequests.length,
      itemBuilder: (context, index) {
        final request = service.pendingRequests[index];
        return Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: AuthCard(
            request: request,
            onApprove: () {
              service.sendDecision(request.requestId, Decision.approved);
            },
            onReject: () {
              service.sendDecision(request.requestId, Decision.rejected);
            },
            onTap: () {
              Navigator.of(context).push(
                MaterialPageRoute(
                  builder: (_) => AuthScreen(request: request),
                ),
              );
            },
          ),
        );
      },
    );
  }
}
