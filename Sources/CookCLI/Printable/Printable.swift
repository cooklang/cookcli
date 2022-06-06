//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift

let OFFSET_UNIT = 4
let MAX_WIDTH = 100

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
    case cookware(Equipment, Int = 0)
    case step(Step, Int, Int = 0)
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
            return ingredients.ingredients.sorted(by: { $0.0 < $1.0 }).map { (k, v) in
                "\(k.padding(toLength: 30, withPad: " ", startingAt: 0) )\(v)".indented(offset)
            }.joined(separator: "\n")
        case .cookware(let cookware, let offset):
            return cookware.description.indented(offset)
        case .step(let step, let index, let offset):
//            TODO estimate max length properly
//            TODO get tty size and don't split by MAX_WIDTH if it's too narrow
            let number = "\(index + 1)".indented(to: 2)
            let directions = "\(number). \(step.directions.map{ $0.description }.joined())"
            var firstLine: String = ""
            var restLines: [String] = []

            for (index, line) in directions.split(byCount: MAX_WIDTH).split(whereSeparator: \.isNewline).enumerated() {
                if index == 0 {
                    firstLine = String(line)
                } else {
                    restLines.append(String(line))
                }
            }
            let ingredients = "[\(step.ingredientsTable.description.split(byCount: MAX_WIDTH))]"

            if restLines.isEmpty {
                return [firstLine.indented(offset), ingredients.indented(offset + 4)].joined(separator: "\n")
            } else {
                return [firstLine.indented(offset), restLines.joined(separator: "\n").indented(offset + 4), ingredients.indented(offset + 4)].joined(separator: "\n")
            }

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
        var output: [Substring] = [];
        for line in self.split(whereSeparator: \.isNewline) {
            output.append(spaces > 0 ? "\((0..<spaces).map { _ in "\(character)" }.joined())\(line)" : line)
        }

        return output.joined(separator: "\n")
    }

    private func split(line: Substring, byCount n: Int, breakableCharacters: [Character]) -> String {
        var line = String(line)
        var lineStartIndex = line.startIndex

        while line.distance(from: lineStartIndex, to: line.endIndex) > n {
            let maxLineEndIndex = line.index(lineStartIndex, offsetBy: n)

            if breakableCharacters.contains(line[maxLineEndIndex]) {
                // If line terminates at a breakable character, replace that character with a newline
                line.replaceSubrange(maxLineEndIndex...maxLineEndIndex, with: "\n")
                lineStartIndex = line.index(after: maxLineEndIndex)
            } else if let index = line[lineStartIndex..<maxLineEndIndex].lastIndex(where: { breakableCharacters.contains($0) }) {
                // Otherwise, find a breakable character that is between lineStartIndex and maxLineEndIndex
                line.replaceSubrange(index...index, with: "\n")
                lineStartIndex = index
            } else {
                // Finally, forcible break a word
                line.insert("\n", at: maxLineEndIndex)
                lineStartIndex = maxLineEndIndex
            }
        }

        return line
    }

    fileprivate func split(byCount n: Int, breakableCharacters: [Character] = [" "]) -> String {
        precondition(n > 0)
        var string = self

        guard !string.isEmpty && string.count > n else { return string }

        var startIndex = string.startIndex

        repeat {
            // Break a string into lines.
            var endIndex = string[string.index(after: startIndex)...].firstIndex(of: "\n") ?? string.endIndex
            if string.distance(from: startIndex, to: endIndex) > n {
                let wrappedLine = split(line: string[startIndex..<endIndex], byCount: n, breakableCharacters: breakableCharacters)
                string.replaceSubrange(startIndex..<endIndex, with: wrappedLine)
                endIndex = string.index(startIndex, offsetBy: wrappedLine.count)
            }

            startIndex = endIndex
        } while startIndex < string.endIndex
        return string
    }
}
