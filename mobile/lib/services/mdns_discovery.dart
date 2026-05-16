import 'dart:async';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:nsd/nsd.dart';

class DiscoveredDevice {
  final String name;
  final String host;
  final int port;

  DiscoveredDevice({
    required this.name,
    required this.host,
    required this.port,
  });
}

class MdnsDiscovery {
  Discovery? _discovery;
  final _devicesController = StreamController<List<DiscoveredDevice>>.broadcast();
  final List<DiscoveredDevice> _devices = [];

  Stream<List<DiscoveredDevice>> get devices => _devicesController.stream;
  List<DiscoveredDevice> get currentDevices => List.unmodifiable(_devices);

  Future<void> startScan() async {
    _devices.clear();
    _devicesController.add(_devices);

    // Web has no mDNS; the UI falls back to manual host:port entry.
    if (kIsWeb) return;

    _discovery = await startDiscovery('_moyuguard._tcp');
    _discovery!.addServiceListener((service, status) {
      if (status == ServiceStatus.found) {
        final host = service.addresses?.firstOrNull?.address;
        final port = service.port;
        final name = service.name ?? 'Unknown';

        if (host != null && port != null) {
          final device = DiscoveredDevice(name: name, host: host, port: port);
          _devices.add(device);
          _devicesController.add(List.from(_devices));
        }
      }
    });
  }

  Future<void> stopScan() async {
    if (_discovery != null) {
      await stopDiscovery(_discovery!);
      _discovery = null;
    }
  }

  void dispose() {
    stopScan();
    _devicesController.close();
  }
}
