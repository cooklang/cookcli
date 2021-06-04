import Foundation
import Vapor

func routes(_ app: Application) throws {
    app.get { req -> EventLoopFuture<View> in
        let pwd = Environment.get("PWD")!
        let url = URL(fileURLWithPath: pwd)

        var files = [String]()
        if let enumerator = FileManager.default.enumerator(at: url, includingPropertiesForKeys: [.isRegularFileKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) {
            for case let fileURL as URL in enumerator {
                do {
                    let fileAttributes = try fileURL.resourceValues(forKeys:[.isRegularFileKey])
                    if fileAttributes.isRegularFile! {
                        files.append(fileURL.lastPathComponent)
                    }
                } catch { print(error, fileURL) }
            }
        }

        return req.view.render("hello", ["files": files])
    }

    app.get("hello") { req -> EventLoopFuture<View> in
        return req.view.render("hello", ["name": "Leaf"])
    }
}
