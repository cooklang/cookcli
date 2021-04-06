//
//  main.swift
//  SPI
//
//  Created by Alexey Dubovskoy on 21/12/2020.
//  Copyright © 2020 Alexey Dubovskoy. All rights reserved.
//

import Foundation
import CookInSwift

if CommandLine.arguments.count == 2 {
    let recipe = try String(contentsOfFile: CommandLine.arguments[1], encoding: String.Encoding.utf8)
    let parser = Parser(recipe)
    let node = parser.parse()
    let analyzer = SemanticAnalyzer()
    let parsed = analyzer.analyze(node: node)
    
    print(CommandLine.arguments[1].replacingOccurrences(of: "\\.cook$", with: "", options: [.regularExpression]))
    print("============================")
    print("Ingredients")
    print(parsed.ingredientsTable)
    print("============================")
    print("Steps")
    
    for (index, step) in parsed.steps.enumerated() {
        print("\(index): \(step)")
    }    
} else {
    print("Usage: SPI prograssm.pas")
}
