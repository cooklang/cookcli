//
//  File.swift
//
//
//  Created by Alexey Dubovskoy on 22/06/2021.
//

import Foundation
import CookInSwift

extension Equipment: Encodable {
    enum CodingKeys: String, CodingKey {
        case name
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        try container.encode(name, forKey: .name)
    }

}
