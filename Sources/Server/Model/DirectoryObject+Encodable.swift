//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

extension DirectoryObject: Encodable {
    enum StaticKeys: String, CodingKey {
        case type
        case children
    }

    struct DirectoryKeys: CodingKey {
        var intValue: Int?

        init?(intValue: Int) {
            return nil
        }

        var stringValue: String
        init?(stringValue: String) {
            self.stringValue = stringValue
        }
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: StaticKeys.self)

        try container.encode("directory", forKey: .type)
        var children = container.nestedContainer(keyedBy: DirectoryKeys.self, forKey: .children)

        try files.forEach { file in
            try children.encode(file, forKey: DirectoryKeys(stringValue: file.name)!)
        }

        try directories.forEach { dir in
            try children.encode(dir, forKey: DirectoryKeys(stringValue: dir.name)!)
        }
    }
}
