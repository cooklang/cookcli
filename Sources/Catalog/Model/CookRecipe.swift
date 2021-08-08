//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 07/08/2021.
//

import Foundation
import CookInSwift

public class CookRecipe {
    internal var parsed: SemanticRecipe

    public init(_ text: String) {
        let parser = Parser(text)
        let node = parser.parse()
        let analyzer = SemanticAnalyzer()
        self.parsed = analyzer.analyze(node: node)
    }

    public var ingredientsTable: IngredientTable {
        get { return parsed.ingredientsTable }
    }

    public var metadata: [String : String] {
        get { return parsed.metadata }
    }

    public var steps: [SemanticStep] {
        get { return parsed.steps }
    }

    public var cookware: [ParsedEquipment] {
        get { return parsed.equipment }
    }
}

