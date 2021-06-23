//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift


public func encodeRecipe(recipe: SemanticRecipe) -> [String : [[String : String]]] {
    var ingredients: [[String: String]] = []
    var cookware: [[String: String]] = []
    var steps: [[String: String]] = []

    recipe.ingredientsTable.ingredients.forEach { ingredient in
        ingredients.append(["name": ingredient.key, "amount": ingredient.value.description])
    }

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
