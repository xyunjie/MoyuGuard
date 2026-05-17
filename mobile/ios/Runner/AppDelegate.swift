import ActivityKit
import Flutter
import UIKit
import UserNotifications

private let kAppGroup   = "group.com.moyuguard.mobile"
private let kDarwinNote = "com.moyuguard.decision"
private let kDecisionKey = "pendingDecision"

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {

    private var liveActivityChannel: FlutterMethodChannel?

    override func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, _ in
            print("[AppDelegate] notification permission: \(granted)")
        }
        registerDarwinObserver()
        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }

    func didInitializeImplicitFlutterEngine(_ engineBridge: any FlutterImplicitEngineBridge) {
        GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
        let messenger = engineBridge.applicationRegistrar.messenger()
        setupLiveActivityChannel(binaryMessenger: messenger)
    }

    // MARK: - URL Scheme (iOS 16 fallback from Link buttons)

    override func application(
        _ application: UIApplication,
        open url: URL,
        options: [UIApplication.OpenURLOptionsKey: Any] = [:]
    ) -> Bool {
        guard url.scheme == "moyuguard" else { return false }
        let action    = url.host ?? ""
        let requestId = url.pathComponents.dropFirst().first ?? ""
        if !requestId.isEmpty, (action == "approve" || action == "reject") {
            forwardDecision(action: action, requestId: requestId)
        }
        return true
    }

    // MARK: - Darwin cross-process notification (iOS 17 AppIntent path)

    private func registerDarwinObserver() {
        let center = CFNotificationCenterGetDarwinNotifyCenter()
        let selfPtr = UnsafeMutableRawPointer(Unmanaged.passRetained(self).toOpaque())
        CFNotificationCenterAddObserver(
            center, selfPtr,
            { _, ptr, _, _, _ in
                guard let ptr else { return }
                Unmanaged<AppDelegate>.fromOpaque(ptr).takeUnretainedValue()
                    .processDecisionFromAppGroup()
            },
            kDarwinNote as CFString, nil, .deliverImmediately
        )
    }

    func processDecisionFromAppGroup() {
        guard let defaults = UserDefaults(suiteName: kAppGroup),
              let dict      = defaults.dictionary(forKey: kDecisionKey),
              let action    = dict["action"]    as? String,
              let requestId = dict["requestId"] as? String else { return }
        defaults.removeObject(forKey: kDecisionKey)
        defaults.synchronize()
        DispatchQueue.main.async { [weak self] in
            self?.forwardDecision(action: action, requestId: requestId)
        }
    }

    private func forwardDecision(action: String, requestId: String) {
        liveActivityChannel?.invokeMethod("handleAction", arguments: [
            "action": action, "requestId": requestId,
        ])
    }

    // MARK: - Live Activity MethodChannel

    private func setupLiveActivityChannel(binaryMessenger: FlutterBinaryMessenger) {
        liveActivityChannel = FlutterMethodChannel(
            name: "moyuguard/live_activity",
            binaryMessenger: binaryMessenger
        )
        liveActivityChannel?.setMethodCallHandler { [weak self] call, result in
            self?.handleLiveActivity(call: call, result: result)
        }
        print("[AppDelegate] live_activity channel registered")
    }

    private func handleLiveActivity(call: FlutterMethodCall, result: FlutterResult) {
        guard #available(iOS 16.2, *) else {
            result(FlutterError(code: "UNSUPPORTED", message: "Live Activity requires iOS 16.2+", details: nil))
            return
        }
        let args      = call.arguments as? [String: Any] ?? [:]
        let count     = args["pendingCount"]  as? Int    ?? 0
        let summary   = args["summary"]       as? String ?? ""
        let risk      = args["risk"]          as? String ?? "low"
        let requestId = args["requestId"]     as? String ?? ""

        switch call.method {
        case "start":
            if let err = LiveActivityManager.shared.start(pendingCount: count, summary: summary, risk: risk, requestId: requestId) {
                result(FlutterError(code: "LA_ERROR", message: err, details: nil))
            } else { result(nil) }
        case "update":
            if let err = LiveActivityManager.shared.update(pendingCount: count, summary: summary, risk: risk, requestId: requestId) {
                result(FlutterError(code: "LA_ERROR", message: err, details: nil))
            } else { result(nil) }
        case "end":
            LiveActivityManager.shared.end(); result(nil)
        default:
            result(FlutterMethodNotImplemented)
        }
    }
}
