//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

import CookInSwift

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

        var ingredients: [[String: String]] = []
        var cookware: [[String: String]] = []
        var steps: [[String: String]] = []

        parsed.ingredientsTable.ingredients.forEach { ingredient in
            ingredients.append(["name": ingredient.key, "amount": ingredient.value.description])
        }

        parsed.equipment.forEach { equipment in
            cookware.append(["name": equipment.name])
        }

        parsed.steps.forEach { step in
            steps.append(["description": step.directions.map{ $0.description }.joined()])
        }

        do {
            let jsonData = try JSONEncoder().encode([
                "ingredients": ingredients,
                "cookware": cookware,
                "steps": steps
            ])
            let jsonString = String(data: jsonData, encoding: .utf8)!

            sendData(Data(jsonString.utf8))
        } catch {
            sendData(Data("error".utf8))
        }
    }
    
}
