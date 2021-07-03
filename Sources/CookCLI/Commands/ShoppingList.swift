//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser
import CookInSwift
import Catalog

extension Cook {

    struct ShoppingList: ParsableCommand {

        @Option(name: .shortAndLong, help: "Specify an aisle.conf file to override shopping list default settings")
        var aisle: String?

        @Argument(help: "File or directory with .cook files")
        var filesOrDirectory: [String]

        @Option(help: "Set the output format to json or yaml (default text)")
        var outputFormat: OutputFormat = .text

        @Flag(help: "Print only the ingredients section of the output")
        var onlyIngredients = false

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Create a shopping list")

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

            guard let files = try? listCookFiles(filesOrDirectory) else {
                print("Error getting files", to: &errStream)

                throw ExitCode.failure
            }

            do {
//                TODO add grouping
                let ingredientTable = try combineShoppingList(files)

                try ingredientTable.print(onlyIngredients: onlyIngredients, outputFormat: outputFormat)
            } catch {
                print(error, to: &errStream)

                throw ExitCode.failure
            }
        }
    }
}
