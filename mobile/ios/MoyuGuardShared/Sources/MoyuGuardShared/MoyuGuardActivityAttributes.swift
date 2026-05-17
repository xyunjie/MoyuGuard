import ActivityKit

public struct MoyuGuardActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        public var pendingCount: Int
        public var latestSummary: String
        public var latestRisk: String
        public var latestRequestId: String  // for approve/reject via URL scheme

        public init(pendingCount: Int, latestSummary: String, latestRisk: String, latestRequestId: String = "") {
            self.pendingCount = pendingCount
            self.latestSummary = latestSummary
            self.latestRisk = latestRisk
            self.latestRequestId = latestRequestId
        }
    }
    public init() {}
}
