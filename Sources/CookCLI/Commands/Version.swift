//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser

extension Cook {

    struct Version: ParsableCommand {

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Show the CookCLI version information")

        func run() throws {
            print("v0.1.3 – in food we trust")
        }
    }
}
