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
                    let parser = Parser(recipe)
                    let node = parser.parse()
                    let analyzer = SemanticAnalyzer()
                    let parsed = analyzer.analyze(node: node)

                    parsed.print(onlyIngredients: onlyIngredients, outputFormat: outputFormat)
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
            static var configuration: CommandConfiguration = CommandConfiguration(abstract: "Download a random image from upsplash.com to match the recipe title (TODO)")

            func run() throws {
                let fileUrl = URL(fileURLWithPath: file)

                let recipeTitle = fileUrl.deletingPathExtension().lastPathComponent

                if let unsplashKey = ProcessInfo.processInfo.environment["COOK_UNSPLASH_ACCESS_KEY"] {
                    let imageUrl = try randomImageUrlByTitle(query: recipeTitle, unsplashKey: unsplashKey)

                    let urls = URL(string: imageUrl)
                    let data = try? Data(contentsOf: urls!) //make sure your image in this url does exist, otherwise unwrap in a if let check / try-catch

                    let destinationPath = fileUrl.deletingLastPathComponent().appendingPathComponent("\(recipeTitle).jpg")

                    print("Saving image to \(destinationPath)")

                    try data?.write(to: destinationPath)
                } else {
                    print("Couldn't find COOK_UNSPLASH_ACCESS_KEY environment variable, please registry for free at https://unsplash.com/documentation#registering-your-application", to: &errStream)
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

func randomImageUrlByTitle(query: String, unsplashKey: String) throws -> String  {
    var urlBuilder = URLComponents(string: "https://api.unsplash.com/photos/random")

    urlBuilder?.queryItems = [
        URLQueryItem(name: "query", value: query),
        URLQueryItem(name: "orientation", value: "landscape"),
    ]

    let url = urlBuilder?.url

    var request = URLRequest(url: url!)
    request.httpMethod = "GET"
    request.setValue("Client-ID \(unsplashKey)", forHTTPHeaderField: "Authorization")

    let semaphore = DispatchSemaphore(value: 0)
    var imageUrl: String?

    URLSession.shared.dataTask(with: request) { (data, response, error) in
        if error != nil {
            print(error, to: &errStream)

            semaphore.signal()
            return
        }

        let responseJSON = try? JSONSerialization.jsonObject(with: data!, options: [])
        if let responseJSON = responseJSON as? [String: Any] {
            imageUrl = (responseJSON["urls"] as! [String: Any])["regular"] as? String
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


