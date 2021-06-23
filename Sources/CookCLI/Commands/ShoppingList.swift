//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser
import CookInSwift

extension Cook {

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
}
