//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 07/08/2021.
//

import Foundation
import CookInSwift
import ConfigParser

public class CookShoppingList {
    let UNDEFINED_AISLE = "OTHER (add new items into aisle.conf)"
    let NO_CONFIG_GROUP = "INGREDIENTS"
    var recipes: [CookRecipe]
    var inflection: CookConfig?
    var aisle: CookConfig?

    public init(recipes: [CookRecipe], inflection: CookConfig?, aisle: CookConfig?) {
        self.recipes = recipes
        self.inflection = inflection
        self.aisle = aisle
    }

    // TODO memoize
    public var sections: [String: IngredientTable] {
        get {
            var sections: [String: IngredientTable] = [:]
            let ingredientTable = recipes.reduce(IngredientTable()) { table, recipe in
                mergeIngredientTables(table, recipe.ingredientsTable)
            }

            ingredientTable.ingredients.forEach { name, amounts in
                var shelf = aisle?.items[name]?.uppercased() ?? UNDEFINED_AISLE

                if aisle == nil {
                    shelf = NO_CONFIG_GROUP
                }

                if sections[shelf] == nil {
                    sections[shelf] = IngredientTable()
                }
                sections[shelf]?.add(name: name, amounts: amounts)
            }

            return sections
        }
    }
}
