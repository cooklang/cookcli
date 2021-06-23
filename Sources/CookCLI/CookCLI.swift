//
//  CookCLI.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import ArgumentParser
#if os(Linux)
    import Glibc
#else
    import Darwin
#endif

public struct StderrOutputStream: TextOutputStream {
    public mutating func write(_ string: String) { fputs(string, stderr) }
}
public var errStream = StderrOutputStream()

enum OutputFormat: String, ExpressibleByArgument {
    case text, json, yaml
}

struct Cook: ParsableCommand {

    @Option(name: .shortAndLong, help: "Specify an aisle.conf file to override shopping list default settings (TODO)")
    var aisle: String?

    @Option(name: .shortAndLong, help: "Specify a units.conf file to override units default settings (TODO)")
    var units: String?

    @Option(name: .shortAndLong, help: "Specify an inflection.conf file to override default inflection settings (TODO)")
    var inflection: String?

    // MARK: ParsableCommand
    static var configuration: CommandConfiguration = CommandConfiguration(abstract: """
        A toolkit for command-line interaction with CookLang text files.
        Documentation can be found at https://cooklang.org/cli/help/ and issues reported at https://github.com/CookLang/CookCLI.
        """,
        subcommands: [
            Recipe.self,
            ShoppingList.self,
            Server.self,
            Fetch.self,
            Version.self
        ]
    )
}
