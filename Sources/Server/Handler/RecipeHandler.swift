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

        let recipe = try! String(contentsOfFile: file, encoding: String.Encoding.utf8)
        let parser = Parser(recipe)
        let node = parser.parse()
        let analyzer = SemanticAnalyzer()
        let parsed = analyzer.analyze(node: node)

        do {
            let jsonData = try JSONEncoder().encode(encodeRecipe(recipe: parsed))
            let jsonString = String(data: jsonData, encoding: .utf8)!

            sendData(Data(jsonString.utf8))
        } catch {
            sendData(Data("error".utf8))
        }
    }
    
}
