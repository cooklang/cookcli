//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 23/06/2021.
//

import Foundation
import ArgumentParser

extension Cook {

    struct Fetch: ParsableCommand {

        // MARK: ParsableCommand
        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Pull recipes from the community recipe repository (TODO)")

        func run() throws {

        }
    }
}
