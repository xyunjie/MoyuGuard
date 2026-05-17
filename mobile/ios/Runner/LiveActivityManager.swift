import ActivityKit
import Foundation
import MoyuGuardShared

// MoyuGuardActivityAttributes is defined in the MoyuGuardShared local package.
// Both Runner and MoyuGuardWidgetExtension import from the same module so
// the Swift type is identical: MoyuGuardShared.MoyuGuardActivityAttributes

@available(iOS 16.2, *)
class LiveActivityManager {
    static let shared = LiveActivityManager()
    private var currentActivity: Activity<MoyuGuardActivityAttributes>?

    private init() {}

    /// Returns nil on success, or an error string on failure.
    func start(pendingCount: Int, summary: String, risk: String, requestId: String) -> String? {
        let authorized = ActivityAuthorizationInfo().areActivitiesEnabled
        if !authorized {
            return "areActivitiesEnabled=false — go to Settings → Notifications → [App] → Live Activities"
        }
        end()
        let state = MoyuGuardActivityAttributes.ContentState(
            pendingCount: pendingCount,
            latestSummary: summary,
            latestRisk: risk,
            latestRequestId: requestId
        )
        let content = ActivityContent(state: state, staleDate: nil)
        do {
            currentActivity = try Activity.request(
                attributes: MoyuGuardActivityAttributes(),
                content: content,
                pushType: nil
            )
            return nil
        } catch {
            return "Activity.request failed: \(error)"
        }
    }

    func update(pendingCount: Int, summary: String, risk: String, requestId: String) -> String? {
        guard let activity = currentActivity else {
            if pendingCount > 0 { return start(pendingCount: pendingCount, summary: summary, risk: risk, requestId: requestId) }
            return nil
        }
        let state = MoyuGuardActivityAttributes.ContentState(
            pendingCount: pendingCount,
            latestSummary: summary,
            latestRisk: risk,
            latestRequestId: requestId
        )
        let content = ActivityContent(state: state, staleDate: nil)
        Task { await activity.update(content) }
        return nil
    }

    func end() {
        guard let activity = currentActivity else { return }
        Task {
            await activity.end(dismissalPolicy: ActivityUIDismissalPolicy.immediate)
            currentActivity = nil
        }
    }
}
