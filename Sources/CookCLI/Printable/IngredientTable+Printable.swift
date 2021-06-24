//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 24/06/2021.
//

import Foundation
import CookInSwift
import Yams
import Catalog


extension IngredientTable {

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
        let jsonData = try JSONEncoder().encode(encodeIngredients(self))

        guard let jsonString = String(data: jsonData, encoding: .utf8) else {
            throw Error.invalidJSON
        }

        Swift.print(jsonString.utf8)
    }

    private func printYaml() throws {
        let yamlData = try YAMLEncoder().encode(encodeIngredients(self))

        Swift.print(yamlData.utf8)
    }

    private func printableLines(onlyIngredients: Bool) -> [PrintableLine] {
        var lines: [PrintableLine] = []
        let fullOutput = !onlyIngredients

        if (fullOutput) {
            lines.append(.text("Ingredients:"))
        }

        let offset = onlyIngredients ? 0 : OFFSET_UNIT
        lines.append(.ingredients(self, offset))

        if (fullOutput) {
            lines.append(.empty)
        }

        return lines
    }
}

extension IngredientTable: Printable {
    func printableLines() -> [PrintableLine] {
        printableLines(onlyIngredients: false)
    }
}
