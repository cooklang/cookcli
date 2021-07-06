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
import CookInSwift

extension Cook {

    struct Server: ParsableCommand {

        @Option(name: .shortAndLong, help: "Specify an aisle.conf file to override shopping list default settings. Cook automatically checks in ./config/aisle.conf and $HOME/.config/cook/aisle.conf")
        var aisle: String?

        @Option(name: .shortAndLong, help: "Set the port on which the webserver should listen")
        var port: Int = 9080

        @Option(name: .shortAndLong, help: "Set the IP to which the server should bind")
        var bind: String = "127.0.0.1"

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web")

        func run() throws {
            var config: CookConfig?

            let configPath = findAisleConfig(aisle)

            if let path = configPath {
                if let text = try? String(contentsOfFile: path, encoding: String.Encoding.utf8) {
//                    TODO add throw
                    let parser = ConfigParser(text)
                    config = parser.parse()
                } else {
                    print("Can't read file \(path)", to: &errStream)

                    throw ExitCode.failure
                }
            }

            do {
                try WebServer(interface: bind, port: port).start(aisle: config)
            } catch {
                print(error, to: &errStream)
                
                throw ExitCode.failure
            }
        }
    }
}
