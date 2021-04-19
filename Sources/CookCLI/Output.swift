//
//  Output.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import CookInSwift

struct TextRecipePrinter {

    func print(_ recipe: SemanticRecipe) -> [String] {
        var lines: [String] = []

        lines.append("===========================")
        lines.append("Ingredients")
        lines.append(recipe.ingredientsTable.description)
        lines.append("============================")
        lines.append("Steps")

        for (index, step) in recipe.steps.enumerated() {
            lines.append("\(index): \(step.directions)")
        }

        return lines
    }
}
