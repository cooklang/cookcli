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

    struct Recipe: ParsableCommand {
        struct Read: ParsableCommand {

            @Argument(help: "A .cook file")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Parse and print a CookLang recipe file")

            @Option(help: "Set the output format to json or yaml (default text)")
            var outputFormat: OutputFormat = .text

            @Flag(help: "Print only the ingredients section of the output")
            var onlyIngredients = false

            func run() throws {
                do {
                    let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
                    let parsed = parseFile(recipe: recipe)

                    try parsed.print(onlyIngredients: onlyIngredients, outputFormat: outputFormat)
                } catch {
                    print(error, to: &errStream)
                    throw ExitCode.failure
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
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Download a random image from upsplash.com to match the recipe title")

            func run() throws {
                let fileUrl = URL(fileURLWithPath: file)

                let recipeTitle = fileUrl.deletingPathExtension().lastPathComponent

                guard  let unsplashKey = ProcessInfo.processInfo.environment["COOK_UNSPLASH_ACCESS_KEY"] else {
                    print("Couldn't find COOK_UNSPLASH_ACCESS_KEY environment variable, please registry for free at https://unsplash.com/documentation#registering-your-application", to: &errStream)
                    throw ExitCode.failure
                }

                guard let urls = try? URL(string: randomImageUrlByTitle(query: recipeTitle, unsplashKey: unsplashKey)) else {
                    print("Error downloading information about random image from Unsplash", to: &errStream)
                    throw ExitCode.failure
                }

                guard let data = try? Data(contentsOf: urls) else {
                    print("Error downloading image from Unsplash", to: &errStream)
                    throw ExitCode.failure
                }

                let destinationPath = fileUrl.deletingLastPathComponent().appendingPathComponent("\(recipeTitle).jpg")
                do {
                    print("Saving image to \(destinationPath)".removingPercentEncoding!)

                    try data.write(to: destinationPath)
                } catch {
                    print(error, to: &errStream)
                    throw ExitCode.failure
                }
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
}
enum ImageFetcherError: Error {
    case errorGettingImage
}


enum Unsplash: Swift.Error {
    case baseUrlError
}


func randomImageUrlByTitle(query: String, unsplashKey: String) throws -> String  {
    var urlBuilder = URLComponents(string: "https://api.unsplash.com/photos/random")!

    urlBuilder.queryItems = [
        URLQueryItem(name: "query", value: query),
        URLQueryItem(name: "orientation", value: "landscape"),
    ]

    var request = URLRequest(url: urlBuilder.url!)
    request.httpMethod = "GET"
    request.setValue("Client-ID \(unsplashKey)", forHTTPHeaderField: "Authorization")

    let semaphore = DispatchSemaphore(value: 0)
    var imageUrl: String?

    URLSession.shared.dataTask(with: request) { (maybeData, response, maybeError) in
        if let error = maybeError {
            print(error, to: &errStream)
            semaphore.signal()

            return
        }

        guard let data = maybeData else {
            print("Empty image content", to: &errStream)

            return
        }

        if let responseJSON = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] {
            guard let urls = responseJSON["urls"] as? [String: Any] else {
                print("Invalid JSON response from Unsplash", to: &errStream)

                return
            }

            imageUrl = urls["regular"] as? String
        }

        semaphore.signal()
    }.resume()

    _ = semaphore.wait(timeout: .distantFuture)

    if imageUrl != nil {
        return imageUrl!
    } else {
        throw ImageFetcherError.errorGettingImage
    }


}


