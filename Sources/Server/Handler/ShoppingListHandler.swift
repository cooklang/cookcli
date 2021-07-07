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

struct ShoppingListHandler {
    enum Error: Swift.Error {
        case problemListingFiles
    }

    var aisle: CookConfig?
    var inflection: CookConfig?

    init(aisle: CookConfig?, inflection: CookConfig?) {
        self.aisle = aisle
        self.inflection = inflection
    }

    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let input = environ["swsgi.input"] as! SWSGIInput

    //        guard environ["HTTP_CONTENT_LENGTH"] != nil else {
    //            // handle error
    //            sendJSON([])
    //            return
    //        }

        JSONReader.read(input) { json in
            do {
                let filesOrDirectory: [String] = (json as! [String]).map { file in
                    return "\(FileManager.default.currentDirectoryPath)/samples/\(file).cook"
                }

                guard let files = try? listCookFiles(filesOrDirectory) else {
                    print("HELPME Error getting '.cook' files, please check permissions or file exist")
                    throw Error.problemListingFiles
                }

                let ingredientTable = try combineShoppingList(files, inflection: inflection)

                let sections = groupShoppingList(ingredients: ingredientTable.ingredients, aisle: aisle)

                var result: [String: [[String : String]] ] = [:]

                sections.sorted(by: { $0.0 < $1.0 }).forEach { section, table in
                    for (ingredient, amounts) in table.ingredients {
                        result[section, default: []].append(["name": ingredient, "amount": amounts.description])
                    }
                }

                let jsonData = try JSONEncoder().encode(result)
                let jsonString = String(data: jsonData, encoding: .utf8)!

                sendData(Data(jsonString.utf8))
            } catch {
                sendData(Data("error \(error)".utf8))
            }

        }
    }
}
