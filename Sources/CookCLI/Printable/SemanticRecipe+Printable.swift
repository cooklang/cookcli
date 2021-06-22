//
//  Output.swift
//
//
//  Created by Alexey Dubovskoy on 19/04/2021.
//

import Foundation
import CookInSwift
import Yams
import Catalog

let OFFSET_UNIT = 4


extension SemanticRecipe {

    func print(onlyIngredients: Bool, outputFormat: OutputFormat) {
        switch outputFormat {
        case .text:
            printText(onlyIngredients: onlyIngredients)
        case .json:
            printJson()
        case .yaml:
            printYaml()
        }
    }

    private func printText(onlyIngredients: Bool) {
        Swift.print(printableLines(onlyIngredients: onlyIngredients).map { line in
            return line.description
        }.joined(separator: .newLine))
    }

    private func printJson() {

        do {
            let jsonData = try JSONEncoder().encode(encodeRecipe(recipe: self))
            let jsonString = String(data: jsonData, encoding: .utf8)!

            Swift.print(jsonString.utf8)
        } catch {
//            TODO
            Swift.print("error")
        }
    }

    private func printYaml() {
        do {
            let yamlData = try YAMLEncoder().encode(encodeRecipe(recipe: self))

            Swift.print(yamlData.utf8)
        } catch {
//            TODO
            Swift.print("error")
        }
    }

    private func printableLines(onlyIngredients: Bool) -> [PrintableLine] {
        var lines: [PrintableLine] = []
        let fullOutput = !onlyIngredients

        if (!metadata.isEmpty && fullOutput) {
            lines.append(.text("Metadata:"))
            lines.append(.metadata(metadata, OFFSET_UNIT))
            lines.append(.empty)
        }

        if (fullOutput) {
            lines.append(.text("Ingredients:"))
        }

        let offset = onlyIngredients ? 0 : OFFSET_UNIT
        lines.append(.ingredients(ingredientsTable, offset))

        if (fullOutput) {
            lines.append(.empty)
        }

        if (!equipment.isEmpty && fullOutput) {
            lines.append(.text("Cookware:"))
            equipment.forEach { e in
                lines.append(.cookware(e, OFFSET_UNIT))
            }
            lines.append(.empty)
        }

        if (fullOutput) {
            lines.append(.text("Steps:"))

            let offset = OFFSET_UNIT
            for (index, step) in steps.enumerated() {
                lines.append(.step(step, index, offset))
            }
        }

        return lines
    }
}

extension SemanticRecipe: Printable {
    func printableLines() -> [PrintableLine] {
        printableLines(onlyIngredients: false)
    }
}
