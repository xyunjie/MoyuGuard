import AppIntents
import Foundation

private let kAppGroup = "group.com.moyuguard.mobile"
private let kDarwinNote = "com.moyuguard.decision"
private let kDecisionKey = "pendingDecision"

@available(iOS 17.0, *)
struct ApproveIntent: LiveActivityIntent {
    static var title: LocalizedStringResource = "允许操作"

    @Parameter(title: "Request ID")
    var requestId: String

    init() { requestId = "" }
    init(requestId: String) { self.requestId = requestId }

    func perform() async throws -> some IntentResult {
        writeDecision(action: "approve", requestId: requestId)
        return .result()
    }
}

@available(iOS 17.0, *)
struct RejectIntent: LiveActivityIntent {
    static var title: LocalizedStringResource = "拒绝操作"

    @Parameter(title: "Request ID")
    var requestId: String

    init() { requestId = "" }
    init(requestId: String) { self.requestId = requestId }

    func perform() async throws -> some IntentResult {
        writeDecision(action: "reject", requestId: requestId)
        return .result()
    }
}

private func writeDecision(action: String, requestId: String) {
    guard let defaults = UserDefaults(suiteName: kAppGroup) else { return }
    defaults.set(["action": action, "requestId": requestId], forKey: kDecisionKey)
    defaults.synchronize()
    // Wake the main app process via Darwin cross-process notification
    CFNotificationCenterPostNotification(
        CFNotificationCenterGetDarwinNotifyCenter(),
        CFNotificationName(kDarwinNote as CFString),
        nil, nil, true
    )
}
