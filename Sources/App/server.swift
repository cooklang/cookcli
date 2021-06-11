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

protocol FileSystemObject {}

class FileObject: Encodable, FileSystemObject, Hashable {
    static func == (lhs: FileObject, rhs: FileObject) -> Bool {
        return lhs.name == rhs.name
    }

    var name: String

    init(name: String) {
        self.name = name
    }

    func hash(into hasher: inout Hasher) {
        return hasher.combine(name)
    }

}

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

    struct ChildrenKeys: CodingKey {
        var intValue: Int?

        init?(intValue: Int) {
            return nil
        }

        var stringValue: String
        init?(stringValue: String) {
            self.stringValue = stringValue
        }
    }

}

extension DirectoryObject: Encodable {
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: DirectoryKeys.self)

        try container.encode("directory", forKey: DirectoryKeys(stringValue: "type")!)
        var children = container.nestedContainer(keyedBy: ChildrenKeys.self, forKey: DirectoryKeys(stringValue: "children")!)

        files.forEach { file in
            try! children.encode(["type" : "file"], forKey: ChildrenKeys(stringValue: file.name)!)
        }

        directories.forEach { dir in
            try! children.encode(dir, forKey: ChildrenKeys(stringValue: dir.name)!)
        }
    }
}

public func startServer() throws {

    let loop = try! SelectorEventLoop(selector: try! SelectSelector())
    let router = Router()
    let server = DefaultHTTPServer(eventLoop: loop, port: 9080, app: router.app)



    router["/api/v1/file_tree"] = DataResponse(
        statusCode: 201,
        statusMessage: "created",
        contentType: "application/xml",
        headers: [("X-Foo-Bar", "My header")]
    ) { _ -> Data in
        let pwd = FileManager.default.currentDirectoryPath
        let url = URL(fileURLWithPath: pwd + "/samples")
        let skipPathComponents = url.pathComponents.count

        let fileTree = DirectoryObject(name: "/")

        if let enumerator = FileManager.default.enumerator(at: url, includingPropertiesForKeys: [.isRegularFileKey, .isDirectoryKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) {
               for case let fileURL as URL in enumerator {
                   do {
                    let fileAttributes = try fileURL.resourceValues(forKeys:[.isRegularFileKey, .isDirectoryKey])

                        if fileAttributes.isRegularFile! {
                            var relativePathComponents = fileURL.pathComponents
                            relativePathComponents.removeFirst(skipPathComponents)

                            if fileURL.pathExtension == "cook" {
                                var pointer = fileTree

                                while relativePathComponents.count > 1 {
                                    let dir = relativePathComponents.removeFirst()

                                    let dirObject = DirectoryObject(name: dir)
                                    pointer.directories.insert(dirObject)
                                    pointer = dirObject
                                }

                                pointer.files.insert(FileObject(name: fileURL.deletingPathExtension().lastPathComponent))
                            }
                        }
                   }
                   catch {
                        print(error, fileURL)

                   }
               }
           }

        do {
            let jsonData = try JSONEncoder().encode(fileTree)
            let jsonString = String(data: jsonData, encoding: .utf8)!

            return Data(jsonString.utf8)
        } catch {
            return Data("error".utf8)
        }





//        return [
//            "Healthy Recipes 2":[
//               "type":"directory",
//               "children":[
//                  "Risotto":[
//                     "type":"file"
//                  ]
//               ]
//            ],
//            "Breakfasts":[
//               "type":"directory",
//               "children":[
//                  "Jamie":[
//                     "type":"directory",
//                     "children":[
//                        "Mexican Style Burrito":[
//                           "type":"file"
//                        ],
//                        "Two chesees omelette":[
//                           "type":"file"
//                        ]
//                     ]
//                  ],
//                  "Irish Breakfast":[
//                     "type":"file",
//                     "image":"/path/to/image.png",
//                     "metadata":[
//
//                     ]
//                  ],
//                  "Shakshuka":[
//                     "type":"file"
//                  ],
//                  "Oats":[
//                     "type":"file"
//                  ]
//               ]
//            ],
//            "Sicilian style lamb chops 2":[
//               "type":"file"
//            ],
//            "Chicken French":[
//               "type":"file"
//            ]
//         ]
    }

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
