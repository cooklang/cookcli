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

extension CookRecipe {

    public enum Error: Swift.Error {
        case invalidJSON
    }
    
    func print(onlyIngredients: Bool, outputFormat: OutputFormat) throws {
        switch outputFormat {
        case .text:
            printText(onlyIngredients: onlyIngredients)
        case .json:
            try printJson()
        case .yaml:
            try printYaml()
        }
    }

    private func printText(onlyIngredients: Bool) {
        Swift.print(printableLines(onlyIngredients: onlyIngredients).map { line in
            return line.description
        }.joined(separator: .newLine))
    }

    private func printJson() throws {
        let jsonData = try JSONEncoder().encode(self)

        guard let jsonString = String(data: jsonData, encoding: .utf8) else {
            throw Error.invalidJSON
        }

        Swift.print(jsonString.utf8)
    }

    private func printYaml() throws {
        let yamlData = try YAMLEncoder().encode(self)

        Swift.print(yamlData.utf8)
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

        if (!cookware.isEmpty && fullOutput) {
            lines.append(.text("Cookware:"))
            cookware.forEach { e in
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

extension CookRecipe: Printable {
    func printableLines() -> [PrintableLine] {
        printableLines(onlyIngredients: false)
    }
}
