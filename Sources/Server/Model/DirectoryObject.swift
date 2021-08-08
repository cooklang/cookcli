//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

class DirectoryObject: FileSystemObject, Hashable, Equatable {
    static func == (lhs: DirectoryObject, rhs: DirectoryObject) -> Bool {
        return lhs.name == rhs.name
    }

    var name: String
    var directories: Set<DirectoryObject> = []
    var files: Set<FileObject> = []

    init(name: String) {
        self.name = name
    }

    func hash(into hasher: inout Hasher) {
        return hasher.combine(name)
    }    

}
