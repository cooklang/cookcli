//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser
import Server

extension Cook {

    struct Server: ParsableCommand {

        @Option(name: .shortAndLong, help: "Set the port on which the webserver should listen (default 8080) (TODO)")
        var port: Int = 8080

        @Option(name: .shortAndLong, help: "Set the IP to which the server should bind (default 127.0.0.1) (TODO)")
        var bind: String = "127.0.0.1"

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web (TODO)")

        func run() throws {
            do {
                let server = WebServer()
                try server.start()
            } catch {
                print(error, to: &errStream)
                throw ExitCode.failure
            }
        }
    }
}
