//
//  Output.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import CookInSwift

struct TextRecipePrinter {

    func print(_ recipe: SemanticRecipe, onlyIngredients: Bool, onlySteps: Bool) -> [String] {
        var lines: [String] = []

        lines.append(recipe.metadata.description)

        if (!onlySteps) {
            lines.append("Ingredients")
            lines.append("+++++++++++")
            lines.append(recipe.ingredientsTable.description)
        }

        if (!onlyIngredients && !onlySteps) {
            lines.append("")
        }

        if (!onlyIngredients) {
            lines.append("Steps")
            lines.append("+++++")

            for (index, step) in recipe.steps.enumerated() {
                lines.append("\(index): \(step.directions.map{ $0.description }.joined())")
            }
        }

        return lines
    }
}
