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

        @Option(name: .shortAndLong, help:
                    """
                    Specify an aisle.conf file to set grouping. Cook automatically checks current directory in ./config/aisle.conf and $HOME/.config/cook/aisle.conf
                    """)
        var aisle: String?

        @Option(name: .shortAndLong, help:
                    """
                    Specify an inflection.conf file to define rules of pluralisation. Cook automatically checks current directory in ./config/inflection.conf and $HOME/.config/cook/inflection.conf
                    """)
        var inflection: String?


        @Option(name: .shortAndLong, help: "Set the port on which the webserver should listen")
        var port: Int = 9080

        @Option(name: .shortAndLong, help: "Set the IP to which the server should bind")
        var bind: String = "127.0.0.1"

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web")

        func run() throws {
            var aisleConfig: CookConfig?
            let aisleConfigPath = findConfigFile(type: "aisle", aisle)

            if let path = aisleConfigPath {
                if let text = try? String(contentsOfFile: path, encoding: String.Encoding.utf8) {
//                    TODO add throw
                    let parser = ConfigParser(text)
                    aisleConfig = parser.parse()
                    print("HELPME Error loading config file at \(path), please check syntax", to: &errStream)
                } else {
                    print("HELPME Can't read aisle config file at \(path). Please check permissions", to: &errStream)

                    throw ExitCode.failure
                }
            }

            var inflectionConfig: CookConfig?
            let inflectionConfigPath = findConfigFile(type: "inflection", aisle)

            if let path = inflectionConfigPath {
                if let text = try? String(contentsOfFile: path, encoding: String.Encoding.utf8) {
//                    TODO add throw
                    let parser = ConfigParser(text)
                    inflectionConfig = parser.parse()
                    print("HELPME Error loading config file at \(path), please check syntax", to: &errStream)
                } else {
                    print("HELPME Can't read inflection config file at \(path). Please check permissions", to: &errStream)

                    throw ExitCode.failure
                }
            }

            do {
                try WebServer(interface: bind, port: port).start(aisle: aisleConfig, inflection: inflectionConfig)
            } catch {
                print(error, to: &errStream)
                
                throw ExitCode.failure
            }
        }
    }
}
