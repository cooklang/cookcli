//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation


struct CatalogHandler {
    var root: String

    init(root: String) {
        self.root = root
    }

    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let url = URL(fileURLWithPath: root)
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

                                let file = FileObject(name: fileURL.deletingPathExtension().lastPathComponent)
                                pointer.files.insert(file)

                                ["jpg", "png"].forEach { format in
                                    let picURL = fileURL.deletingPathExtension().appendingPathExtension(format)

                                    if let _ = try? picURL.checkResourceIsReachable() {
                                        file.image = picURL.lastPathComponent
                                    }
                                }
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

            sendData(Data(jsonString.utf8))
        } catch {
            sendData(Data("error".utf8))
        }
    }

}
