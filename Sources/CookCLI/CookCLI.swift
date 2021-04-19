//
//  CookCLI.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import ArgumentParser

import CookInSwift

struct CookCLI: ParsableCommand {

    struct Read: ParsableCommand {

        @Argument(help: "Set cook file")
        var file: String

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Read cook file and dispay it.")

        func run() throws {

            let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
            let parser = Parser(recipe)
            let node = parser.parse()
            let analyzer = SemanticAnalyzer()
            let parsed = analyzer.analyze(node: node)

            let printer = TextRecipePrinter()

            printer.print(parsed).forEach { line in
                print(line)
            }
        }
    }

    struct Version: ParsableCommand {
        func run() throws {
            print("v0.0.1 – in food we trust")
        }
    }

    // MARK: ParsableCommand
    static var configuration: CommandConfiguration = CommandConfiguration(abstract: "A Swift command-line tool to manage recipes",
        discussion: "Requires a ",
        subcommands: [
            Read.self,
            Version.self
        ]
    )
}
