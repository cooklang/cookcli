//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

class FileObject: FileSystemObject, Hashable {
    static func == (lhs: FileObject, rhs: FileObject) -> Bool {
        return lhs.name == rhs.name
    }

    var name: String
    var image: String?

    init(name: String) {
        self.name = name
    }

    func hash(into hasher: inout Hasher) {
        return hasher.combine(name)
    }

}
