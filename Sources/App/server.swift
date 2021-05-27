//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Vapor

public func startServer() throws {
    var env = Environment(name: "development", arguments: ["cook"])
    try LoggingSystem.bootstrap(from: &env)
    print(env)
    let app = Application(env)
    defer { app.shutdown() }
    try configure(app)
    try app.run()
}
