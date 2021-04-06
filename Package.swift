// swift-tools-version:5.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "CookCLI",
//    platforms: [
//        .macOS(.v10_15),
//    ],
    products: [
        .executable(name: "cook", targets: ["CookCLI"])
    ],
    dependencies: [
        // Dependencies declare other packages that this package depends on.
        // .package(url: /* package url */, from: "1.0.0"),
        .package(url: "https://github.com/cooklang/CookInSwift", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-argument-parser", from: "0.0.1")
    ],
    targets: [
        // Targets are the basic building blocks of a package. A target can define a module or a test suite.
        // Targets can depend on other targets in this package, and on products in packages this package depends on.
        .target(
            name: "CookCLI",
            dependencies: ["CookInSwift", .product(name: "ArgumentParser", package: "swift-argument-parser")]),
        .testTarget(
            name: "CookCLITests",
            dependencies: ["CookCLI"]),
    ]
)
