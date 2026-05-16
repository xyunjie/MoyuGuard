import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/connection_service.dart';
import 'screens/scan_screen.dart';
import 'screens/home_screen.dart';
import 'screens/pairing_screen.dart';

void main() {
  runApp(
    ChangeNotifierProvider(
      create: (_) => ConnectionService(),
      child: const MoyuGuardApp(),
    ),
  );
}

class MoyuGuardApp extends StatelessWidget {
  const MoyuGuardApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '摸鱼守卫',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        colorSchemeSeed: Colors.blue,
        useMaterial3: true,
        scaffoldBackgroundColor: const Color(0xFF0F0F0F),
        cardTheme: const CardThemeData(
          color: Color(0xFF222222),
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radius.circular(12)),
            side: BorderSide(color: Color(0xFF333333)),
          ),
        ),
      ),
      home: Consumer<ConnectionService>(
        builder: (context, service, _) {
          if (!service.isConnected) {
            return const ScanScreen();
          }
          if (service.pairState == PairState.waiting) {
            return const PairingScreen();
          }
          if (service.pairState == PairState.rejected) {
            return const PairingScreen(rejected: true);
          }
          return const HomeScreen();
        },
      ),
    );
  }
}
