//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift

// Kitchen sink, TODO

public func encodeRecipe(_ recipe: SemanticRecipe) -> [String : [[String : String]]] {
    var cookware: [[String: String]] = []
    var steps: [[String: String]] = []

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
        "ingredients": encodeIngredients(recipe.ingredientsTable),
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



public func combineShoppingList(_ files: [String], inflection: CookConfig?) throws -> IngredientTable {
    var ingredientTable = IngredientTable()

    try files.forEach { file in
        let recipe = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
        let parsed = parseFile(recipe: recipe)

        ingredientTable = ingredientTable + parsed.ingredientsTable
    }

    return ingredientTable
}

public func groupShoppingList(ingredients:  [String: IngredientAmountCollection], aisle: CookConfig?) -> [String: IngredientTable] {
    var sections: [String: IngredientTable] = [:]

    ingredients.forEach { name, amounts in
        var shelf = aisle?.items[name]?.uppercased() ?? "OTHER (add new items into aisle.conf)"

        if aisle == nil {
            shelf = "INGREDIENTS"
        }

        if sections[shelf] == nil {
            sections[shelf] = IngredientTable()
        }
        sections[shelf]?.add(name: name, amounts: amounts)
    }

    return sections
}

public func findConfigFile(type: String, _ provided: String?) -> String? {
    var configPath: String?

    let local = FileManager.default.currentDirectoryPath + "/config/\(type).conf"
    let home = FileManager.default.homeDirectoryForCurrentUser.path + "/.config/cook/\(type).conf"

    if provided != nil {
        configPath = provided
    } else if FileManager.default.fileExists(atPath: local) {
        configPath = local
    } else if FileManager.default.fileExists(atPath: home) {
        configPath = home
    }

    return configPath
}

fileprivate func directoryExistsAtPath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(true)
    let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
    return exists && isDirectory.boolValue
}
