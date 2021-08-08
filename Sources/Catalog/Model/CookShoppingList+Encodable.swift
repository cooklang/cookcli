//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 08/08/2021.
//

import Foundation


extension CookShoppingList: Encodable {

    enum CodingKeys: String, CodingKey {
        case description
    }

    struct SectionKeys: CodingKey {
        var intValue: Int?

        init?(intValue: Int) {
            return nil
        }

        var stringValue: String
        init?(stringValue: String) {
            self.stringValue = stringValue
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: SectionKeys.self)

        try sections.sorted(by: { $0.0 < $1.0 }).forEach { section, table in
            try container.encode(table, forKey: SectionKeys(stringValue: section)!)
        }
    }
}
