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


        @Argument(help: "File or directory with .cook files to include to shopping list")
        var filesOrDirectory: [String]

        @Option(help: "Set the output format to json or yaml")
        var outputFormat: OutputFormat = .text

        @Flag(help: "Print only the ingredients section of the output")
        var onlyIngredients = false

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Create a shopping list")

        func run() throws {
            let configLoader = ConfigLoader()
            let fileLister = CookFileLister()
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

            guard let files = try? fileLister.list(filesOrDirectory) else {
                print("Could not read .cook files at \(filesOrDirectory). Make sure the files exist, and that you have permission to read them.", to: &errStream)

                throw ExitCode.failure
            }

            let recipes: [CookRecipe] = try files.map { file in
                if let text = try? String(contentsOfFile: file, encoding: String.Encoding.utf8) {
                    return CookRecipe(text)
                } else {
                    print("Could not read .cook files at \(file). Make sure the file exist, and that you have permission to read them.", to: &errStream)

                    throw ExitCode.failure
                }
            }

            do {
                let shoppingList = CookShoppingList(recipes: recipes, inflection: inflectionConfig, aisle: aisleConfig)

                try shoppingList.print(onlyIngredients: onlyIngredients, outputFormat: outputFormat)
            } catch {
                print(error, to: &errStream)

                throw ExitCode.failure
            }
        }
    }
}
