import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/connection_service.dart';

class PairingScreen extends StatelessWidget {
  final bool rejected;
  const PairingScreen({super.key, this.rejected = false});

  @override
  Widget build(BuildContext context) {
    final service = context.read<ConnectionService>();

    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Spacer(),
              AnimatedSwitcher(
                duration: const Duration(milliseconds: 300),
                child: rejected
                    ? const Icon(Icons.cancel_outlined, size: 72, color: Color(0xFFef4444), key: ValueKey('rejected'))
                    : _PulsingIcon(key: const ValueKey('waiting')),
              ),
              const SizedBox(height: 28),
              Text(
                rejected ? '配对被拒绝' : '等待电脑确认',
                style: const TextStyle(fontSize: 22, fontWeight: FontWeight.bold, color: Colors.white),
              ),
              const SizedBox(height: 12),
              Text(
                rejected
                    ? '电脑端点击了"拒绝"。请联系电脑持有者，或重新发起连接。'
                    : '电脑端正在弹出配对确认弹窗，\n请在电脑上点击"允许配对"。',
                textAlign: TextAlign.center,
                style: const TextStyle(fontSize: 14, color: Color(0xFF888888), height: 1.6),
              ),
              const SizedBox(height: 12),
              if (!rejected)
                Text(
                  service.serverAddress ?? '',
                  style: const TextStyle(fontSize: 12, color: Color(0xFF555555), fontFamily: 'monospace'),
                ),
              const Spacer(),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton(
                  onPressed: () => service.disconnect(),
                  style: OutlinedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 14),
                    side: const BorderSide(color: Color(0xFF444444)),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                  child: Text(
                    rejected ? '重新连接' : '取消',
                    style: const TextStyle(color: Color(0xFF888888)),
                  ),
                ),
              ),
              const SizedBox(height: 16),
            ],
          ),
        ),
      ),
    );
  }
}

class _PulsingIcon extends StatefulWidget {
  const _PulsingIcon({super.key});

  @override
  State<_PulsingIcon> createState() => _PulsingIconState();
}

class _PulsingIconState extends State<_PulsingIcon> with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl;
  late final Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(vsync: this, duration: const Duration(milliseconds: 1200))
      ..repeat(reverse: true);
    _scale = Tween<double>(begin: 0.92, end: 1.08).animate(
      CurvedAnimation(parent: _ctrl, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ScaleTransition(
      scale: _scale,
      child: Container(
        width: 80,
        height: 80,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: const Color(0xFF2563eb).withValues(alpha: 0.15),
          border: Border.all(color: const Color(0xFF2563eb), width: 2),
        ),
        child: const Icon(Icons.smartphone, size: 36, color: Color(0xFF2563eb)),
      ),
    );
  }
}
