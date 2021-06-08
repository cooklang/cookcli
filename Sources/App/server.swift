//
//  server.swift
//  
//
//  Created by Alexey Dubovskoy on 27/05/2021.
//

import Embassy
import Ambassador
import Foundation
import CookInSwift

public func startServer() throws {

    let loop = try! SelectorEventLoop(selector: try! SelectSelector())
    let router = Router()
    let server = DefaultHTTPServer(eventLoop: loop, port: 9080, app: router.app)

    router["/api/v1/file_tree"] = JSONResponse(handler: { _ -> Any in
        let pwd = FileManager.default.currentDirectoryPath
        let url = URL(fileURLWithPath: pwd + "/samples")

        var fileTree: [String: Any] = [:]

        if let enumerator = FileManager.default.enumerator(at: url, includingPropertiesForKeys: [.isRegularFileKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) {
               for case let fileURL as URL in enumerator {
                   do {
                       let fileAttributes = try fileURL.resourceValues(forKeys:[.isRegularFileKey])
                       if fileAttributes.isRegularFile! {
//                           files.append(fileURL.lastPathComponent)
                            print(fileURL.lastPathComponent)
                       }
                   }
                   catch {
                        print(error, fileURL)

                   }
               }
           }

        return [
            "Healthy Recipes 2":[
               "type":"directory",
               "children":[
                  "Risotto":[
                     "type":"file"
                  ]
               ]
            ],
            "Breakfasts":[
               "type":"directory",
               "children":[
                  "Jamie":[
                     "type":"directory",
                     "children":[
                        "Mexican Style Burrito":[
                           "type":"file"
                        ],
                        "Two chesees omelette":[
                           "type":"file"
                        ]
                     ]
                  ],
                  "Irish Breakfast":[
                     "type":"file",
                     "image":"/path/to/image.png",
                     "metadata":[

                     ]
                  ],
                  "Shakshuka":[
                     "type":"file"
                  ],
                  "Oats":[
                     "type":"file"
                  ]
               ]
            ],
            "Sicilian style lamb chops 2":[
               "type":"file"
            ],
            "Chicken French":[
               "type":"file"
            ]
         ]
    })

    router["/api/v1/recipe/(.+)"] = JSONResponse(handler: { environ -> Any in
        let captures = environ["ambassador.router_captures"] as! [String]
        var path = captures[0]
        path = path.removingPercentEncoding!

        let pwd = FileManager.default.currentDirectoryPath
        let file = "\(pwd)/samples/\(path).cook"

        let recipe = try! String(contentsOfFile: file, encoding: String.Encoding.utf8)
        let parser = Parser(recipe)
        let node = parser.parse()
        let analyzer = SemanticAnalyzer()
        let parsed = analyzer.analyze(node: node)

        var ingredients: [[String: String]] = []
        var cookware: [[String: String]] = []
        var steps: [[String: String]] = []

        parsed.ingredientsTable.ingredients.forEach { ingredient in
            ingredients.append(["name": ingredient.key, "amount": ingredient.value.description])
        }

        parsed.equipment.forEach { equipment in
            cookware.append(["name": equipment.name])
        }

        parsed.steps.forEach { step in
            steps.append(["description": step.directions.map{ $0.description }.joined()])
        }

        var response: [String: Any] = [
            "ingredients": ingredients,
            "cookware": cookware,
            "steps": steps
        ]

        return response
    })

    // Start HTTP server to listen on the port
    try! server.start()

    print("started server on http://localhost:9080")
    // Run event loop
    loop.runForever()

}
