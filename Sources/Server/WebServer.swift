//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Embassy
import Ambassador
import CookInSwift

public struct WebServer {
    var interface: String
    var port: Int

    public init(interface: String = "::1", port: Int = 9080) {
        self.interface = interface
        self.port = port
    }

    public func start(aisle: CookConfig?, inflection: CookConfig?) throws {

        let loop = try SelectorEventLoop(selector: try SelectSelector())
        let router = Router()

        let server = DefaultHTTPServer(eventLoop: loop, interface: interface, port: port, app: router.app)

        router["^/api/v1/file_tree"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: CatalogHandler().callAsFunction)
        router["^/api/v1/recipe/(.+)"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: RecipeHandler().callAsFunction)
        router["^/api/v1/shopping-list"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: ShoppingListHandler(aisle: aisle, inflection: inflection).callAsFunction)

        router["^/favicon.png"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "text/html; charset=UTF-8", headers: [("Content-Encoding", "br")], handler: StaticAssetsHandler.FaviconPng().callAsFunction)

        router["^/build/bundle.js"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/javascript; charset=UTF-8", headers: [("Content-Encoding", "br")], handler: StaticAssetsHandler.BundleJs().callAsFunction)

        router["^/build/bundle.css"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "text/css; charset=UTF-8", headers: [("Content-Encoding", "br")], handler: StaticAssetsHandler.BundleCss().callAsFunction)

        router["^/vendor/bootstrap/css/bootstrap.min.css"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "text/css; charset=UTF-8", headers: [("Content-Encoding", "br")], handler: StaticAssetsHandler.BootstrapCss().callAsFunction)

        router["^/(.+jpg)$"] = DataResponse(statusCode: 200, contentType: "image/jpeg", handler: FileSystemAssetsHandler().callAsFunction)
        router["^/(.+png)$"] = DataResponse(statusCode: 200, contentType: "image/png", handler: FileSystemAssetsHandler().callAsFunction)

        router["^/$"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "text/html; charset=UTF-8", headers: [("Content-Encoding", "br")], handler: StaticAssetsHandler.IndexHTML().callAsFunction)

        // Start HTTP server to listen on the port
        try server.start()

        print("Started server on http://\(interface):\(port)")

        loop.runForever()
    }
}
