import 'dart:async';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/connection_service.dart';
import '../services/mdns_discovery.dart';

class ScanScreen extends StatefulWidget {
  const ScanScreen({super.key});

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  final _mdns = MdnsDiscovery();
  final _hostController = TextEditingController(text: kIsWeb ? '127.0.0.1' : '');
  final _portController = TextEditingController(text: '9876');
  bool _isScanning = false;
  bool _isConnecting = false;
  List<DiscoveredDevice> _devices = [];
  StreamSubscription? _sub;

  @override
  void initState() {
    super.initState();
    _startScan();
  }

  Future<void> _startScan() async {
    if (kIsWeb) return;
    setState(() => _isScanning = true);
    try {
      await _mdns.startScan();
      _sub = _mdns.devices.listen((devices) {
        if (mounted) {
          setState(() => _devices = devices);
        }
      });
    } catch (e) {
      debugPrint('mDNS scan failed: $e');
    }
  }

  Future<void> _connect(String host, int port) async {
    setState(() => _isConnecting = true);
    final service = context.read<ConnectionService>();
    await service.connect(host, port);
    if (mounted) setState(() => _isConnecting = false);
  }

  Future<void> _manualConnect() async {
    final host = _hostController.text.trim();
    final port = int.tryParse(_portController.text.trim()) ?? 9876;
    if (host.isEmpty) return;
    await _connect(host, port);
  }

  @override
  void dispose() {
    _sub?.cancel();
    _mdns.dispose();
    _hostController.dispose();
    _portController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SizedBox(height: 40),
              const Text(
                '🐟 摸鱼守卫',
                style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 8),
              Text(
                kIsWeb ? '请手动输入电脑端 IP 和端口' : '扫描局域网中的电脑...',
                style: TextStyle(
                  fontSize: 14,
                  color: Colors.grey[500],
                ),
              ),
              const SizedBox(height: 32),

              if (_devices.isNotEmpty) ...[
                Text(
                  '发现的设备',
                  style: TextStyle(
                    fontSize: 13,
                    color: Colors.grey[400],
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                ..._devices.map(
                  (device) => Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Card(
                      child: ListTile(
                        leading: const Icon(Icons.computer, color: Colors.blue),
                        title: Text(device.name),
                        subtitle: Text('${device.host}:${device.port}'),
                        trailing: _isConnecting
                            ? const SizedBox(
                                width: 20,
                                height: 20,
                                child: CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.arrow_forward_ios, size: 16),
                        onTap: _isConnecting
                            ? null
                            : () => _connect(device.host, device.port),
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 24),
              ],

              if (_isScanning && _devices.isEmpty)
                const Center(
                  child: Padding(
                    padding: EdgeInsets.symmetric(vertical: 40),
                    child: Column(
                      children: [
                        CircularProgressIndicator(),
                        SizedBox(height: 16),
                        Text('正在扫描...', style: TextStyle(color: Colors.grey)),
                      ],
                    ),
                  ),
                ),

              const Spacer(),

              Text(
                '手动连接',
                style: TextStyle(
                  fontSize: 13,
                  color: Colors.grey[400],
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    flex: 3,
                    child: TextField(
                      controller: _hostController,
                      decoration: InputDecoration(
                        hintText: 'IP 地址',
                        filled: true,
                        fillColor: const Color(0xFF222222),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: const BorderSide(color: Color(0xFF333333)),
                        ),
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 12,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    flex: 1,
                    child: TextField(
                      controller: _portController,
                      keyboardType: TextInputType.number,
                      decoration: InputDecoration(
                        hintText: '端口',
                        filled: true,
                        fillColor: const Color(0xFF222222),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: const BorderSide(color: Color(0xFF333333)),
                        ),
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 12,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  SizedBox(
                    height: 48,
                    child: ElevatedButton(
                      onPressed: _isConnecting ? null : _manualConnect,
                      child: const Text('连接'),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 24),
            ],
          ),
        ),
      ),
    );
  }
}
