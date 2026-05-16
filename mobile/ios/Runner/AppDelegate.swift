import ActivityKit
import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {

    override func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }

    func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
        GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
        setupLiveActivityChannel(binaryMessenger: engineBridge.binaryMessenger)
    }

    private func setupLiveActivityChannel(binaryMessenger: FlutterBinaryMessenger) {
        let channel = FlutterMethodChannel(
            name: "moyuguard/live_activity",
            binaryMessenger: binaryMessenger
        )
        channel.setMethodCallHandler { [weak self] call, result in
            self?.handleLiveActivity(call: call, result: result)
        }
    }

    private func handleLiveActivity(call: FlutterMethodCall, result: FlutterResult) {
        guard #available(iOS 16.2, *) else {
            result(FlutterError(code: "UNSUPPORTED", message: "Live Activity requires iOS 16.2+", details: nil))
            return
        }

        let args = call.arguments as? [String: Any] ?? [:]
        let count   = args["pendingCount"]  as? Int    ?? 0
        let summary = args["summary"]       as? String ?? ""
        let risk    = args["risk"]          as? String ?? "low"

        switch call.method {
        case "start":
            LiveActivityManager.shared.start(pendingCount: count, summary: summary, risk: risk)
            result(nil)
        case "update":
            LiveActivityManager.shared.update(pendingCount: count, summary: summary, risk: risk)
            result(nil)
        case "end":
            LiveActivityManager.shared.end()
            result(nil)
        default:
            result(FlutterMethodNotImplemented)
        }
    }
}
