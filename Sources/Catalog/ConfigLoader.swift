//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 07/08/2021.
//

import Foundation
import CookInSwift
import ConfigParser

public enum ConfigLoadError: Error {
    case UnparsableFile(path: String)
    case UnreadableFile(path: String)
}

public class ConfigLoader {

    public init() {}

    public func load(type: String, referenced: String?) throws -> CookConfig? {
        var config: CookConfig?
        let maybePath = findConfigFile(type: type, referenced)

        if let path = maybePath {
            if let text = try? String(contentsOfFile: path, encoding: String.Encoding.utf8) {
                // TODO add throw
                let parser = ConfigParser(text)
                config = parser.parse()
//                throw ConfigLoadError.UnparsableFile(path: path)
            } else {
                throw ConfigLoadError.UnreadableFile(path: path)
            }
        }

        return config
    }

    private func findConfigFile(type: String, _ provided: String?) -> String? {
        var configPath: String?
        let configFileName = "\(type).conf"
        // TODO handle path separator nicely and cross-platform
        let local = FileManager.default.currentDirectoryPath + "/config/" + configFileName
        let home = "~/.config/cook/" + configFileName

        if provided != nil {
            configPath = provided
        } else if FileManager.default.fileExists(atPath: local) {
            configPath = local
        } else if FileManager.default.fileExists(atPath: home) {
            configPath = home
        }

        return configPath
    }

}
