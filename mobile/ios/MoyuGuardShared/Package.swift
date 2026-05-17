// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MoyuGuardShared",
    platforms: [.iOS(.v13)],
    products: [
        .library(name: "MoyuGuardShared", targets: ["MoyuGuardShared"])
    ],
    dependencies: [],
    targets: [
        .target(name: "MoyuGuardShared", path: "Sources/MoyuGuardShared")
    ]
)
