//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

extension FileObject: Encodable {

    enum FileKeys: String, CodingKey {
        case type
        case image
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: FileKeys.self)

        try container.encode("file", forKey: .type)

        if let i = image {
            try container.encode(i, forKey: .image)
        }
    }
}
