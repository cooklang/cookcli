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

        @Argument(help: "A path to serve cook files from")
        var root: String?

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web")

        func run() throws {
            let configLoader = ConfigLoader()
            var aisleConfig: CookConfig?
            var inflectionConfig: CookConfig?

            do {
                aisleConfig = try configLoader.load(type: "aisle", referenced: aisle)
            } catch ConfigLoadError.UnparsableFile(let path) {
                print("Could not parse aisle config file at \(path). Make sure the syntax of the config file is correct.", to: &errStream)
            } catch ConfigLoadError.UnreadableFile(let path) {
                print("Could not read aisle config file at \(path). Make sure the file exists, and that you have permission to read it.", to: &errStream)

                throw ExitCode.failure
            }

            do {
                inflectionConfig = try configLoader.load(type: "inflection", referenced: aisle)
            } catch ConfigLoadError.UnparsableFile(let path) {
                print("Could not parse inflection config file at \(path). Make sure the syntax of the config file is correct.", to: &errStream)
            } catch ConfigLoadError.UnreadableFile(let path) {
                print("Could not read inflection config file at \(path). Make sure the file exists, and that you have permission to read it.", to: &errStream)

                throw ExitCode.failure
            }

            do {
                try WebServer(interface: bind, port: port).start(root: root ?? FileManager.default.currentDirectoryPath, aisle: aisleConfig, inflection: inflectionConfig)
            } catch {
                print(error, to: &errStream)
                
                throw ExitCode.failure
            }
        }
    }
}
