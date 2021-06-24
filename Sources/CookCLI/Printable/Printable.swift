//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift

let OFFSET_UNIT = 4

protocol Printable {
    func printableLines() -> [PrintableLine]
}

extension Printable {
    func print() {
        Swift.print(printableLines().map { line in
            return line.description
        }.joined(separator: .newLine))
    }
}

enum PrintableLine: CustomStringConvertible {
    case label(PrintableLabel, String? = nil)
    case metadata([String: String], Int = 0)
    case ingredients(IngredientTable, Int = 0)
    case cookware(ParsedEquipment, Int = 0)
    case step(SemanticStep, Int, Int = 0)
    case text(String)
    case empty
    case offset(String, Int)

    var description: String {
        switch self {
        case .label(let label, let string):
            return label.description.joined(with: string)
        case .metadata(let metadata, let offset):
            return metadata.map { (k, v) in
                "\(k): \(v)".indented(offset)
            }.joined(separator: "\n")
        case .ingredients(let ingredients, let offset):
//            TODO estimate max length properly
            return ingredients.ingredients.map { (k, v) in
                "\(k.padding(toLength: 30, withPad: " ", startingAt: 0) )\(v)".indented(offset)
            }.joined(separator: "\n")
        case .cookware(let cookware, let offset):
            return cookware.description.indented(offset)
        case .step(let step, let index, let offset):
//            TODO estimate max length properly
            let number = "\(index + 1)".indented(to: 2)
            let firstLine = "\(number). \(step.directions.map{ $0.description }.joined())"
            let secondLine = "[\(step.ingredientsTable.description)]"
            return [firstLine.indented(offset), secondLine.indented(offset + 4)].joined(separator: "\n")
        case .text(let string):
            return string
        case .offset(let string, let offset):
            return string.indented(to: offset)
        case .empty:
            return ""
        }
    }
}

enum PrintableLabel: ExpressibleByStringLiteral, CustomStringConvertible {
    case custom(String), options, error

    // MARK: ExpressibleByStringLiteral
    typealias StringLiteralType = String

    init(stringLiteral value: String) {
        switch value.lowercased() {
        case "options":
            self = .options
        case "error":
            self = .error
        default:
            self = .custom(value)
        }
    }

    // MARK: CustomStringConvertible
    var description: String {
        switch self {
        case .custom(let string):
            return !string.isEmpty ? "\(string.uppercased()):" : ""
        case .options:
            return "OPTIONS:"
        case .error:
            return "ERROR:"
        }
    }
}

extension Array: Printable where Element == PrintableLine {

    // MARK: Printable
    func printableLines() -> [PrintableLine] {
        return self
    }
}

extension String {
    static let `default`: String = "(default)"
    static let newLine: String = "\n"
    static let bullet: String = "•"

    var redacted: String {
        return map { _ in
            return "\(Self.bullet)"
        }.joined()
    }

    fileprivate func merged(with string: String?, indented spaces: Int, character: Character = " ") -> String {
        guard let string: String = string, !string.isEmpty else {
            return self
        }
        guard self.count < spaces else {
            return [self, string.indented(spaces)].joined(separator: .newLine)
        }
        return "\(self)\(string.indented(spaces - self.count, character: character))"
    }

    fileprivate func joined(with string: String?) -> String {
        return "\(self)\(string?.indented(1) ?? "")"
    }

    fileprivate func indented(to length: Int, character: Character = " ") -> String {
        return indented(max(length - count, 0), character: character)
    }

    fileprivate func indented(_ spaces: Int, character: Character = " ") -> String {
        return spaces > 0 ? "\((0..<spaces).map { _ in "\(character)" }.joined())\(self)" : self
    }
}
