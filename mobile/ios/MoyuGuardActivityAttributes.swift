import ActivityKit

// This file MUST be added to both Runner and MoyuGuardWidgetExtension targets.
// ActivityKit matches Live Activity types by the exact Swift module-qualified name,
// so both the app and the widget extension must compile from the same source file.
struct MoyuGuardActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var pendingCount: Int
        var latestSummary: String
        var latestRisk: String
    }
}
