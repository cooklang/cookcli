//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 07/08/2021.
//

import Foundation
import CookInSwift

public class CookRecipe {
    internal var parsed: Recipe

    public init(_ text: String) {
        self.parsed = try! Recipe.from(text: text)
    }

    public var ingredientsTable: IngredientTable {
        get { return parsed.ingredientsTable }
    }

    public var metadata: [String : String] {
        get { return parsed.metadata }
    }

    public var steps: [Step] {
        get { return parsed.steps }
    }

    public var cookware: [Equipment] {
        get { return parsed.equipment }
    }
}

