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

func readSTDIN () -> String? {
    var input: String?

    while let line = readLine() {
        if input == nil {
            input = line
        } else {
            input! += "\n" + line
        }
    }

    return input
}

enum OutputFormat: String, ExpressibleByArgument {
    case text, json, yaml
}

struct Cook: ParsableCommand {    

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
            Seed.self,
            Version.self
        ]
    )
}
