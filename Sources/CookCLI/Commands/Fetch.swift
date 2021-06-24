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

        @Argument(help: "Path")
        var communityRecipePath: String?

        static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Pull recipes from the community recipe repository")

        func run() throws {
            var path: String

            if let c = communityRecipePath {
                path = c
            } else {
                guard let input = readSTDIN() else {
                    print("Path for community recipe not provided, set recipe name or pass in STDIN", to: &errStream)
                    throw ExitCode.failure
                }

                path = input
            }

            if !path.hasSuffix(".cook") {
                path = "\(path).cook"
            }

            guard let raw = "https://raw.githubusercontent.com/cooklang/recipes/main/\(path)".addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
                  let url = URL(string: raw) else {

                print("Invalid path", to: &errStream)
                throw ExitCode.failure
            }

            guard let data = try? Data(contentsOf: url) else {
                print("Error downloading recipe from \(url), please check that path is correct", to: &errStream)
                throw ExitCode.failure
            }

            do {
                let pwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
                let destinationPath = pwd.appendingPathComponent(url.lastPathComponent)

                print("Saving recipe to \(destinationPath)".removingPercentEncoding!)

                try data.write(to: destinationPath)
            } catch {
                print(error, to: &errStream)

                throw ExitCode.failure
            }

        }
    }
}
