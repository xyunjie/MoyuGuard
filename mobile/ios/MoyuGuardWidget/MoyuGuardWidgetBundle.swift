import WidgetKit
import SwiftUI

@main
struct MoyuGuardWidgetBundle: WidgetBundle {
    var body: some Widget {
        if #available(iOS 16.2, *) {
            MoyuGuardWidgetLiveActivity()
        }
    }
}
