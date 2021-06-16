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

struct ShoppingListHandler {
    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let input = environ["swsgi.input"] as! SWSGIInput

    //        guard environ["HTTP_CONTENT_LENGTH"] != nil else {
    //            // handle error
    //            sendJSON([])
    //            return
    //        }

        JSONReader.read(input) { json in
            // handle the json object here
            do {
                var ingredientTable = IngredientTable()

                var files: [String]
                print("hello")
                let filesOrDirectory: [String] = (json as! [String]).map { file in
                    return "\(FileManager.default.currentDirectoryPath)/samples/\(file)"
                }

                print(filesOrDirectory)

                if filesOrDirectory.count == 1 && directoryExistsAtPath(filesOrDirectory[0]) {
                    let directory = filesOrDirectory[0]
                    let directoryContents = try FileManager.default.contentsOfDirectory(atPath: directory)
                    files = directoryContents.filter{ $0.hasSuffix("cook") }.map { "\(directory)/\($0)" }
                } else {
                    files = filesOrDirectory
                }


                try files.forEach { file in
                    let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
                    let parser = Parser(recipe)
                    let node = parser.parse()
                    let analyzer = SemanticAnalyzer()
                    let parsed = analyzer.analyze(node: node)

                    ingredientTable = ingredientTable + parsed.ingredientsTable
                }

                var result: [[String: String]] = []
                for (ingredient, amounts) in ingredientTable.ingredients {
                    result.append(["name": ingredient, "amount": amounts.description])
                }

                let jsonData = try JSONEncoder().encode(result)
                let jsonString = String(data: jsonData, encoding: .utf8)!

                sendData(Data(jsonString.utf8))
            } catch {
                sendData(Data("error".utf8))
            }

        }
    }

    fileprivate func directoryExistsAtPath(_ path: String) -> Bool {
        var isDirectory = ObjCBool(true)
        let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
        return exists && isDirectory.boolValue
    }
}
