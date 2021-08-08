//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation

import CookInSwift
import Catalog

struct RecipeHandler {

    var root: String

    init(root: String) {
        self.root = root
    }

    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let captures = environ["ambassador.router_captures"] as! [String]
        var path = captures[0]
        path = path.removingPercentEncoding!

        let file = "\(root)/\(path).cook"

        do {
            let text = try String(contentsOfFile: file, encoding: String.Encoding.utf8)
            let recipe = CookRecipe(text)
//            TODO include pic, title, etc
            let jsonData = try JSONEncoder().encode(recipe)
            let jsonString = String(data: jsonData, encoding: .utf8)!

            sendData(Data(jsonString.utf8))
        } catch {
            sendData(Data("error".utf8))
        }
    }
    
}
