//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

import CookInSwift
import Catalog

struct RecipeHandler {

    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let captures = environ["ambassador.router_captures"] as! [String]
        var path = captures[0]
        path = path.removingPercentEncoding!

        let pwd = FileManager.default.currentDirectoryPath
        let file = "\(pwd)/samples/\(path).cook"

        do {
            let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
            let parsed = parseFile(recipe: recipe)
//            TODO need to have intermidiate wrapper for recipe to include pic, title, etc
            let jsonData = try JSONEncoder().encode(encodeRecipe(parsed))
            let jsonString = String(data: jsonData, encoding: .utf8)!

            sendData(Data(jsonString.utf8))
        } catch {
            sendData(Data("error".utf8))
        }
    }
    
}
