// Package.swift — Swift package wrapping the Rust staticlib + SwiftUI editor.
import PackageDescription

let package = Package(
    name: "PicsartClone",
    platforms: [.iOS(.v17)],
    products: [.library(name: "PicsartClone", targets: ["PicsartClone"])],
    targets: [
        .target(name: "PicsartClone", path: "Sources"),
        // Build libeditor_core.a via: cargo build -p picsart-editor-core --release --target aarch64-apple-ios
        // and add it under .binaryTarget in the Xcode project (see README).
    ]
)
