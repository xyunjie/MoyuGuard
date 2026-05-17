//
//  MoyuGuardWidgetLiveActivity.swift
//  MoyuGuardWidget
//

import ActivityKit
import AppIntents
import MoyuGuardShared
import SwiftUI
import WidgetKit

// MoyuGuardActivityAttributes comes from MoyuGuardShared package.
// Same module = same Swift type = ActivityKit can match correctly.

// ── Risk helpers ─────────────────────────────────────────────────────────────

private extension MoyuGuardActivityAttributes.ContentState {
    var riskColor: Color {
        switch latestRisk {
        case "critical": return .red
        case "high":     return .orange
        case "medium":   return Color(red: 1, green: 0.8, blue: 0)
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

// ── Live Activity ─────────────────────────────────────────────────────────────

struct MoyuGuardWidgetLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: MoyuGuardActivityAttributes.self) { context in
            // Lock screen / notification banner — with approve/reject buttons
            let rid = context.state.latestRequestId
            VStack(spacing: 10) {
                HStack(spacing: 12) {
                    Text("🐟").font(.title2)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(context.state.pendingCount) 个待审批")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(.white)
                        Text(context.state.latestSummary)
                            .font(.system(size: 12))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    Text(context.state.riskLabel)
                        .font(.system(size: 11, weight: .medium))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(context.state.riskColor.opacity(0.2))
                        .foregroundStyle(context.state.riskColor)
                        .clipShape(Capsule())
                }
                if !rid.isEmpty {
                    HStack(spacing: 12) {
                        if #available(iOS 17.0, *) {
                            Button(intent: RejectIntent(requestId: rid)) {
                                Label("拒绝", systemImage: "xmark")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(.red)
                                    .frame(maxWidth: .infinity)
                                    .padding(.vertical, 8)
                                    .background(.red.opacity(0.15))
                                    .clipShape(RoundedRectangle(cornerRadius: 10))
                            }
                            .buttonStyle(.plain)
                            Button(intent: ApproveIntent(requestId: rid)) {
                                Label("允许", systemImage: "checkmark")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(.green)
                                    .frame(maxWidth: .infinity)
                                    .padding(.vertical, 8)
                                    .background(.green.opacity(0.15))
                                    .clipShape(RoundedRectangle(cornerRadius: 10))
                            }
                            .buttonStyle(.plain)
                        } else {
                            Link(destination: URL(string: "moyuguard://reject/\(rid)")!) {
                                Label("拒绝", systemImage: "xmark")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(.red)
                                    .frame(maxWidth: .infinity)
                                    .padding(.vertical, 8)
                                    .background(.red.opacity(0.15))
                                    .clipShape(RoundedRectangle(cornerRadius: 10))
                            }
                            Link(destination: URL(string: "moyuguard://approve/\(rid)")!) {
                                Label("允许", systemImage: "checkmark")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(.green)
                                    .frame(maxWidth: .infinity)
                                    .padding(.vertical, 8)
                                    .background(.green.opacity(0.15))
                                    .clipShape(RoundedRectangle(cornerRadius: 10))
                            }
                        }
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .activityBackgroundTint(.black.opacity(0.85))

        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    HStack(spacing: 6) {
                        Text("🐟").font(.title3)
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
                HStack(spacing: 3) {
                    Text("🐟").font(.system(size: 13))
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
                Circle()
                    .fill(context.state.riskColor)
                    .frame(width: 8, height: 8)
            } minimal: {
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
