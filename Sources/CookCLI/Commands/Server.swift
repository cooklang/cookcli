//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser
import Server
import Catalog

extension Cook {

    struct Server: ParsableCommand {

        @Option(name: .shortAndLong, help: "Specify an aisle.conf file to override shopping list default settings")
        var aisle: String?

        @Option(name: .shortAndLong, help: "Set the port on which the webserver should listen (default 8080)")
        var port: Int = 9080

        @Option(name: .shortAndLong, help: "Set the IP to which the server should bind (default 127.0.0.1)")
        var bind: String = "127.0.0.1"

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web")

        func run() throws {
            let configPath = findAisleConfig(aisle)

            do {
                try WebServer(interface: bind, port: port).start()
            } catch {
                print(error, to: &errStream)
                
                throw ExitCode.failure
            }
        }
    }
}
