//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Embassy
import Ambassador
import Foundation

public func startServer() throws {



    let loop = try! SelectorEventLoop(selector: try! SelectSelector())
    let router = Router()
    let server = DefaultHTTPServer(eventLoop: loop, interface: "::", port: 9080, app: router.app)

    router["/api/v2/users"] = DelayResponse(JSONResponse(handler: { _ -> Any in
        return [
            ["id": "01", "name": "john"],
            ["id": "02", "name": "tom"]
        ]
    }))

    // Start HTTP server to listen on the port
    try! server.start()

    print("started server on http://localhost:9080")
    // Run event loop
    loop.runForever()

}
