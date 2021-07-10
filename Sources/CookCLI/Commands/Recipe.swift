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

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

extension Cook {

    struct Recipe: ParsableCommand {
        struct Read: ParsableCommand {

            @Argument(help: "A .cook file or STDIN")
            var recipeFile: String?

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Parse and print a CookLang recipe file")

            @Option(help: "Set the output format to json or yaml")
            var outputFormat: OutputFormat = .text

            @Flag(help: "Print only the ingredients section of the output")
            var onlyIngredients = false

            func run() throws {
                var recipe: String

                if let file = recipeFile {
                    if let r = try? String(contentsOfFile: file, encoding: String.Encoding.utf8) {
                        recipe = r
                    } else {
                        print("Could not read recipe file at \(file). Make sure the file exists, and that you have permission to read it.", to: &errStream)

                        throw ExitCode.failure
                    }
                } else {
                    if let r = readSTDIN() {
                        recipe = r
                    }  else {
                        print("Missing recipe name or path. Set the recipe name with cook recipe RECIPE. \nYou can also pass STDIN.", to: &errStream)

                        throw ExitCode.failure
                    }
                }

                do {
                    let parsed = parseFile(recipe: recipe)

                    try parsed.print(onlyIngredients: onlyIngredients, outputFormat: outputFormat)
                } catch {
                    print(error, to: &errStream)

                    throw ExitCode.failure
                }

            }
        }

        struct Validate: ParsableCommand {

            @Argument(help: "A .cook file or STDIN")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Check for syntax errors in one or more CookLang recipe files (TODO)")

            func run() throws {

            }
        }

        struct Prettify: ParsableCommand {

            @Argument(help: "A .cook file or STDIN")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Edit a CookLang recipe file for style consistency (TODO)")

            func run() throws {

            }
        }

        struct Image: ParsableCommand {

            @Argument(help: "A .cook file or STDIN")
            var file: String

            // MARK: ParsableCommand
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Download a random image from upsplash.com to match the recipe title")

            func run() throws {
                let fileUrl = URL(fileURLWithPath: file)

                let recipeTitle = fileUrl.deletingPathExtension().lastPathComponent

                guard  let unsplashKey = ProcessInfo.processInfo.environment["COOK_UNSPLASH_ACCESS_KEY"] else {
                    print("Could not find COOK_UNSPLASH_ACCESS_KEY environment variable, please register for free at https://unsplash.com/documentation#registering-your-application and set environment variable.", to: &errStream)

                    throw ExitCode.failure
                }

                guard let urls = try? URL(string: randomImageUrlByTitle(query: recipeTitle, unsplashKey: unsplashKey)) else {
                    print("Could not connect to Unsplash. Make sure your access key is valid, and that you have access to the internet.", to: &errStream)

                    throw ExitCode.failure
                }

                guard let data = try? Data(contentsOf: urls) else {
                    print("Could not download image from Unsplash. Make sure you have access to the internet.", to: &errStream)
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
            print("Image download returned empty image. Try again to look for a new random image.", to: &errStream)

            return
        }

        if let responseJSON = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] {
            guard let urls = responseJSON["urls"] as? [String: Any] else {
                print("Invalid JSON response from Unsplash. Try again to look for a new random image.", to: &errStream)

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
