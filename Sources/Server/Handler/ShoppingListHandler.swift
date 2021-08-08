//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

import Embassy
import Ambassador
import CookInSwift
import Catalog

public enum ShoppingListHandlerError: Error {
    case FailedListingFiles
    case FailedReadingFile(path: String)
}

struct ShoppingListHandler {
    var root: String
    var aisle: CookConfig?
    var inflection: CookConfig?
    var fileLister = CookFileLister()

    init(root: String, aisle: CookConfig?, inflection: CookConfig?) {
        self.root = root
        self.aisle = aisle
        self.inflection = inflection
    }

    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let input = environ["swsgi.input"] as! SWSGIInput

        JSONReader.read(input) { json in
            do {
                let filesOrDirectory: [String] = (json as! [String]).map { file in
                    return "\(root)/\(file).cook"
                }

                guard let files = try? fileLister.list(filesOrDirectory) else {
                    print("Error reading '.cook' files. Make sure the files exist, and that you have permission to read them.")
                    throw ShoppingListHandlerError.FailedListingFiles
                }

                let recipes: [CookRecipe] = try files.map { file in
                    if let text = try? String(contentsOfFile: file, encoding: String.Encoding.utf8) {
                        return CookRecipe(text)
                    } else {
                        throw ShoppingListHandlerError.FailedReadingFile(path: file)
                    }
                }

                let shoppingList = CookShoppingList(recipes: recipes, inflection: inflection, aisle: aisle)

                let jsonData = try JSONEncoder().encode(shoppingList)
                let jsonString = String(data: jsonData, encoding: .utf8)!

                sendData(Data(jsonString.utf8))
            } catch {
                sendData(Data("error \(error)".utf8))
            }

        }
    }
}
