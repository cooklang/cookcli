//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Embassy
import Ambassador
import CookInSwift
import ConfigParser

public struct WebServer {
    var interface: String
    var port: Int

    public init(interface: String = "::1", port: Int = 9080) {
        self.interface = interface
        self.port = port
    }

    public func start(root: String, aisle: CookConfig?, inflection: CookConfig?) throws {

        let loop = try SelectorEventLoop(selector: try SelectSelector())
        let router = Router()

        let server = DefaultHTTPServer(eventLoop: loop, interface: interface, port: port, app: router.app)

        // TODO return non 200 on error
        router["^/api/v1/file_tree"] = DataResponse(contentType: "application/json", handler: CatalogHandler(root: root).callAsFunction)
        router["^/api/v1/recipe/(.+)"] = DataResponse(contentType: "application/json", handler: RecipeHandler(root: root).callAsFunction)
        router["^/api/v1/shopping-list"] = DataResponse(contentType: "application/json", handler: ShoppingListHandler(root: root, aisle: aisle, inflection: inflection).callAsFunction)

        router["^/favicon.png"] = DataResponse(contentType: "text/html; charset=UTF-8", headers: [("Content-Encoding", "gzip")], handler: StaticAssetsHandler.FaviconPng().callAsFunction)

        router["^/build/bundle.js"] = DataResponse(contentType: "application/javascript; charset=UTF-8", headers: [("Content-Encoding", "gzip")], handler: StaticAssetsHandler.BundleJs().callAsFunction)

        router["^/build/bundle.css"] = DataResponse(contentType: "text/css; charset=UTF-8", headers: [("Content-Encoding", "gzip")], handler: StaticAssetsHandler.BundleCss().callAsFunction)

        router["^/vendor/bootstrap/css/bootstrap.min.css"] = DataResponse(contentType: "text/css; charset=UTF-8", headers: [("Content-Encoding", "gzip")], handler: StaticAssetsHandler.BootstrapCss().callAsFunction)

        router["^/(.+jpg)$"] = DataResponse(contentType: "image/jpeg", handler: FileSystemAssetsHandler(root: root).callAsFunction)
        router["^/(.+png)$"] = DataResponse(contentType: "image/png", handler: FileSystemAssetsHandler(root: root).callAsFunction)

        router.notFoundResponse = DataResponse(contentType: "text/html; charset=UTF-8", headers: [("Content-Encoding", "gzip")], handler: StaticAssetsHandler.IndexHTML().callAsFunction)

        // Start HTTP server to listen on the port
        try server.start()

        print("Started server on http://\(interface):\(port), serving cook files from \(root).")

        loop.runForever()
    }
}
