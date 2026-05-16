import ActivityKit
import Foundation

// Must match the definition in MoyuGuardWidgetLiveActivity.swift
struct MoyuGuardActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var pendingCount: Int
        var latestSummary: String
        var latestRisk: String
    }
}

@available(iOS 16.2, *)
class LiveActivityManager {
    static let shared = LiveActivityManager()
    private var currentActivity: Activity<MoyuGuardActivityAttributes>?

    private init() {}

    func start(pendingCount: Int, summary: String, risk: String) {
        guard ActivityAuthorizationInfo().areActivitiesEnabled else { return }
        end()

        let state = MoyuGuardActivityAttributes.ContentState(
            pendingCount: pendingCount,
            latestSummary: summary,
            latestRisk: risk
        )
        let attrs = MoyuGuardActivityAttributes()
        let content = ActivityContent(state: state, staleDate: nil)

        do {
            currentActivity = try Activity.request(
                attributes: attrs,
                content: content,
                pushType: nil
            )
        } catch {
            print("[LiveActivity] start failed: \(error)")
        }
    }

    func update(pendingCount: Int, summary: String, risk: String) {
        guard let activity = currentActivity else {
            if pendingCount > 0 { start(pendingCount: pendingCount, summary: summary, risk: risk) }
            return
        }
        let state = MoyuGuardActivityAttributes.ContentState(
            pendingCount: pendingCount,
            latestSummary: summary,
            latestRisk: risk
        )
        let content = ActivityContent(state: state, staleDate: nil)
        Task { await activity.update(content) }
    }

    func end() {
        guard let activity = currentActivity else { return }
        Task {
            await activity.end(dismissalPolicy: ActivityUIDismissalPolicy.immediate)
            currentActivity = nil
        }
    }
}
