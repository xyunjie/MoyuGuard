import ActivityKit
import SwiftUI
import WidgetKit

// ── Data contract (shared with Runner) ──────────────────────────────────────

struct MoyuGuardActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var pendingCount: Int
        var latestSummary: String
        var latestRisk: String   // "low" | "medium" | "high" | "critical"
    }
}

// ── Risk helpers ─────────────────────────────────────────────────────────────

private extension MoyuGuardActivityAttributes.ContentState {
    var riskColor: Color {
        switch latestRisk {
        case "critical": return .red
        case "high":     return .orange
        case "medium":   return .yellow
        default:         return .green
        }
    }

    var riskLabel: String {
        switch latestRisk {
        case "critical": return "危险"
        case "high":     return "高风险"
        case "medium":   return "中风险"
        default:         return "低风险"
        }
    }
}

// ── Live Activity View ───────────────────────────────────────────────────────

@available(iOS 16.2, *)
struct MoyuGuardLiveActivityView: View {
    let state: MoyuGuardActivityAttributes.ContentState

    var body: some View {
        HStack(spacing: 12) {
            Text("🐟")
                .font(.title2)

            VStack(alignment: .leading, spacing: 2) {
                Text("\(state.pendingCount) 个待审批")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(.white)

                Text(state.latestSummary)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            Text(state.riskLabel)
                .font(.system(size: 11, weight: .medium))
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(state.riskColor.opacity(0.2))
                .foregroundStyle(state.riskColor)
                .clipShape(Capsule())
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

// ── Widget ───────────────────────────────────────────────────────────────────

@available(iOS 16.2, *)
struct MoyuGuardWidgetLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: MoyuGuardActivityAttributes.self) { context in
            // Lock screen / banner
            MoyuGuardLiveActivityView(state: context.state)
                .background(.black.opacity(0.85))

        } dynamicIsland: { context in
            DynamicIsland {
                // Expanded (long-press)
                DynamicIslandExpandedRegion(.leading) {
                    HStack(spacing: 6) {
                        Text("🐟")
                            .font(.title3)
                        Text("摸鱼守卫")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(.white)
                    }
                }

                DynamicIslandExpandedRegion(.trailing) {
                    Text(context.state.riskLabel)
                        .font(.system(size: 11, weight: .medium))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(context.state.riskColor.opacity(0.2))
                        .foregroundStyle(context.state.riskColor)
                        .clipShape(Capsule())
                }

                DynamicIslandExpandedRegion(.bottom) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("\(context.state.pendingCount) 个操作等待审批")
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(.white)
                        Text(context.state.latestSummary)
                            .font(.system(size: 12))
                            .foregroundStyle(.secondary)
                            .lineLimit(2)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, 4)
                }

            } compactLeading: {
                // Compact leading: fish + count badge
                HStack(spacing: 3) {
                    Text("🐟")
                        .font(.system(size: 13))
                    ZStack {
                        Circle()
                            .fill(context.state.riskColor)
                            .frame(width: 16, height: 16)
                        Text("\(context.state.pendingCount)")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundStyle(.black)
                    }
                }

            } compactTrailing: {
                // Compact trailing: risk dot
                Circle()
                    .fill(context.state.riskColor)
                    .frame(width: 8, height: 8)

            } minimal: {
                // Minimal (when two activities compete)
                ZStack {
                    Circle()
                        .fill(context.state.riskColor)
                        .frame(width: 20, height: 20)
                    Text("\(context.state.pendingCount)")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundStyle(.black)
                }
            }
            .widgetURL(URL(string: "moyuguard://pending"))
            .keylineTint(context.state.riskColor)
        }
    }
}
