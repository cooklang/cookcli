import Foundation
import CookInSwift


extension IngredientTable: Encodable {

    enum CodingKeys: String, CodingKey {
        case name
        case amount
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()

        try ingredients.forEach { i in
            var ingredient = container.nestedContainer(keyedBy: CodingKeys.self)

            try ingredient.encode(i.key, forKey: .name)
            try ingredient.encode(i.value.description, forKey: .amount)
        }
    }
}
