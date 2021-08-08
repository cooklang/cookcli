//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 08/08/2021.
//

import Foundation
import CookInSwift

public class CookFileLister {

    public init() {}

    public func list(_ filesOrDirectory: [String]) throws -> [String] {
        return try listCookFiles(filesOrDirectory)
    }

    private func listCookFiles(_ filesOrDirectory: [String]) throws -> [String] {
        if filesOrDirectory.count == 1 && directoryExistsAtPath(filesOrDirectory[0]) {
            let directory = filesOrDirectory[0]
            let directoryContents = try FileManager.default.contentsOfDirectory(atPath: directory)

            return directoryContents.filter{ $0.hasSuffix("cook") }.map { "\(directory)/\($0)" }
        } else {
            return filesOrDirectory
        }
    }

    private func directoryExistsAtPath(_ path: String) -> Bool {
        var isDirectory = ObjCBool(true)
        let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
        return exists && isDirectory.boolValue
    }
}
