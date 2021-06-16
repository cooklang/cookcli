//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Embassy
import Ambassador

public struct WebServer {
    public init() {}

    public func start() throws {

        let loop = try SelectorEventLoop(selector: try! SelectSelector())
        let router = Router()

        let server = DefaultHTTPServer(eventLoop: loop, port: 9080, app: router.app)

        router["/api/v1/file_tree"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: CatalogHandler().callAsFunction)
        router["/api/v1/recipe/(.+)"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: RecipeHandler().callAsFunction)
        router["/api/v1/shopping-list"] = DataResponse(statusCode: 200, statusMessage: "ok", contentType: "application/json", handler: ShoppingListHandler().callAsFunction)

        // Start HTTP server to listen on the port
        try server.start()

        print("Started server on http://localhost:9080")

        loop.runForever()
    }
}
