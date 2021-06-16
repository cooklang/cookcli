//
//  CookCLI.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import ArgumentParser
import Server
import CookInSwift

struct Cook: ParsableCommand {
    enum OutputFormat: String, ExpressibleByArgument {
        case text, json, yaml
    }

    struct Recipe: ParsableCommand {
        struct Read: ParsableCommand {

            @Argument(help: "A .cook file")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Parse and print a CookLang recipe file")

            @Option(help: "Set the output format to json or yaml (default text) (TODO)")
            var outputFormat: OutputFormat?

            @Flag(help: "Print only the steps section of the output")
            var onlySteps = false

            @Flag(help: "Print only the ingredients section of the output")
            var onlyIngredients = false

            @Flag(help: "Print a machine-friendly version of the output (TODO)")
            var compact = false

            func run() throws {
                let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
                let parser = Parser(recipe)
                let node = parser.parse()
                let analyzer = SemanticAnalyzer()
                let parsed = analyzer.analyze(node: node)

                let printer = TextRecipePrinter()

                printer.print(parsed, onlyIngredients: onlyIngredients, onlySteps: onlySteps).forEach { line in
                    print(line)
                }
            }
        }

        struct Validate: ParsableCommand {

            @Argument(help: "A .cook file")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Check for syntax errors in one or more CookLang recipe files (TODO)")

            func run() throws {

            }
        }

        struct Prettify: ParsableCommand {

            @Argument(help: "A .cook file")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Edit a CookLang recipe file for style consistency (TODO)")

            func run() throws {

            }
        }

        struct Image: ParsableCommand {

            @Argument(help: "A .cook file")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Download a random image from upsplash.com to match the recipe title (TODO)")

            func run() throws {

            }
        }

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Manage recipes and recipe files",
            subcommands: [
                Read.self,
                Validate.self,
                Prettify.self,
                Image.self,
            ]
        )
    }

    struct ShoppingList: ParsableCommand {

        @Argument(help: "File or directory with .cook files")
        var filesOrDirectory: [String]

        @Option(help: "Set the output format to json or yaml (default text) (TODO)")
        var outputFormat: OutputFormat?

        @Flag(help: "Print only the ingredients section of the output (TODO)")
        var onlyIngredients = false

        @Flag(help: "Print a machine-friendly version of the output (TODO)")
        var compact = false

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Create a shopping list")

        func run() throws {
            var ingredientTable = IngredientTable()

            var files: [String]

            if filesOrDirectory.count == 1 && directoryExistsAtPath(filesOrDirectory[0]) {
                let directory = filesOrDirectory[0]
                let directoryContents = try FileManager.default.contentsOfDirectory(atPath: directory)
                files = directoryContents.filter{ $0.hasSuffix("cook") }.map { "\(directory)/\($0)" }
            } else {
                files = filesOrDirectory
            }


            try files.forEach { file in
                let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
                let parser = Parser(recipe)
                let node = parser.parse()
                let analyzer = SemanticAnalyzer()
                let parsed = analyzer.analyze(node: node)

                ingredientTable = ingredientTable + parsed.ingredientsTable
            }

            for (ingredient, amounts) in ingredientTable.ingredients {
                print(ingredient.padding(toLength: 30, withPad: " ", startingAt: 0), "\t", amounts.description)
            }

        }

        fileprivate func directoryExistsAtPath(_ path: String) -> Bool {
            var isDirectory = ObjCBool(true)
            let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
            return exists && isDirectory.boolValue
        }
    }

    struct Server: ParsableCommand {

        @Option(name: .shortAndLong, help: "Set the port on which the webserver should listen (default 8080) (TODO)")
        var port: Int = 8080

        @Option(name: .shortAndLong, help: "Set the IP to which the server should bind (default 127.0.0.1) (TODO)")
        var bind: String = "127.0.0.1"

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Run a webserver to serve your recipes on the web (TODO)")

        func run() throws {
            let server = WebServer()
            try server.start()
        }
    }

    struct Fetch: ParsableCommand {

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Pull recipes from the community recipe repository (TODO)")

        func run() throws {

        }
    }

    struct Version: ParsableCommand {

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Show the CookCLI version information (TODO)")

        func run() throws {
            print("v0.0.1 – in food we trust")
        }
    }

    @Option(name: .shortAndLong, help: "Specify an aisle.conf file to override shopping list default settings (TODO)")
    var aisle: String?

    @Option(name: .shortAndLong, help: "Specify a units.conf file to override units default settings (TODO)")
    var units: String?

    @Option(name: .shortAndLong, help: "Specify an inflection.conf file to override default inflection settings (TODO)")
    var inflection: String?

    // MARK: ParsableCommand
    static var configuration: CommandConfiguration = CommandConfiguration(abstract: "A toolkit for command-line interaction with CookLang text files",
        subcommands: [
            Recipe.self,
            ShoppingList.self,
            Server.self,
            Fetch.self,
            Version.self
        ]
    )


}
