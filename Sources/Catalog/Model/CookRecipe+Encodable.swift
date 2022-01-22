//
//  File.swift
//
//
//  Created by Alexey Dubovskoy on 07/08/2021.
//

import Foundation
import CookInSwift

extension CookRecipe: Encodable {

    enum CodingKeys: String, CodingKey {
        case metadata
        case ingredients
        case cookware
        case steps
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        try container.encode(parsed.metadata, forKey: .metadata)
        try container.encode(parsed.ingredientsTable, forKey: .ingredients)
        try container.encode(parsed.equipment, forKey: .cookware)
        try container.encode(parsed.steps, forKey: .steps)
    }
}
