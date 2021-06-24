//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift


public func encodeRecipe(_ recipe: SemanticRecipe) -> [String : [[String : String]]] {
    var cookware: [[String: String]] = []
    var steps: [[String: String]] = []
    var ingredients: [[String: String]] = encodeIngredients(recipe.ingredientsTable)

    recipe.equipment.forEach { equipment in
        cookware.append(["name": equipment.name])
    }

    recipe.steps.forEach { step in
//        TODO
//        var ingredients: [[String: String]] = []
//        step.ingredientsTable.ingredients.forEach { ingredient in
//            ingredients.append(["name": ingredient.key, "amount": ingredient.value.description])
//        }
        steps.append(["description": step.directions.map{ $0.description }.joined()])
    }

    return [
        "ingredients": ingredients,
        "cookware": cookware,
        "steps": steps
    ]
}

public func encodeIngredients(_ ingredientsTable: IngredientTable) -> [[String : String]] {
    var ingredients: [[String: String]] = []

    ingredientsTable.ingredients.forEach { ingredient in
        ingredients.append(["name": ingredient.key, "amount": ingredient.value.description])
    }

    return ingredients
}


public func parseFile(recipe: String) -> SemanticRecipe {    
    let parser = Parser(recipe)
    let node = parser.parse()
    let analyzer = SemanticAnalyzer()
    return analyzer.analyze(node: node)
}

public func listCookFiles(_ filesOrDirectory: [String]) throws -> [String] {
    if filesOrDirectory.count == 1 && directoryExistsAtPath(filesOrDirectory[0]) {
        let directory = filesOrDirectory[0]
        let directoryContents = try FileManager.default.contentsOfDirectory(atPath: directory)

        return directoryContents.filter{ $0.hasSuffix("cook") }.map { "\(directory)/\($0)" }
    } else {
        return filesOrDirectory
    }
}

public func combineShoppingList(_ files: [String]) throws -> IngredientTable {
    var ingredientTable = IngredientTable()

    try files.forEach { file in
        let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
        let parsed = parseFile(recipe: recipe)

        ingredientTable = ingredientTable + parsed.ingredientsTable
    }

    return ingredientTable
}

fileprivate func directoryExistsAtPath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(true)
    let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
    return exists && isDirectory.boolValue
}
