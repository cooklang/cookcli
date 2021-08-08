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

extension CookShoppingList {

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
        sections.sorted(by: { $0.0 < $1.0 }).forEach { section, table in
            Swift.print(printableLines(table: table, onlyIngredients: onlyIngredients, title: section).map { $0.description }.joined(separator: .newLine))
        }
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

    private func printableLines(table: IngredientTable, onlyIngredients: Bool, title: String?) -> [PrintableLine] {
        var lines: [PrintableLine] = []
        let fullOutput = !onlyIngredients

        if (fullOutput) {
            if let t = title {
                lines.append(.text(t))
            }
        }

        let offset = onlyIngredients ? 0 : OFFSET_UNIT
        lines.append(.ingredients(table, offset))

        if (fullOutput) {
            lines.append(.empty)
        }

        return lines
    }

}

//extension CookShoppingList: Printable {
//    func printableLines() -> [PrintableLine] {
//        printableLines(table: self, onlyIngredients: false, title: "Ingredients:")
//    }
//}
