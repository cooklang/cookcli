//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 30/06/2021.
//

import Foundation

//
//  File.swift
//
//
//  Created by Alexey Dubovskoy on 28/06/2021.
//

import Foundation
//
//  File.swift
//
//
//  Created by Alexey Dubovskoy on 16/06/2021.
//

import Foundation


struct FileSystemAssetsHandler {


    func callAsFunction(_ environ: [String : Any], _ sendData: @escaping (Data) -> Void) -> Void {
        let captures = environ["ambassador.router_captures"] as! [String]
        var path = captures[0]
        path = path.removingPercentEncoding!

        let pwd = FileManager.default.currentDirectoryPath
//        TODO
        let file = "\(pwd)/samples/\(path)"

        do {
            try sendData(Data(contentsOf: URL(fileURLWithPath: file)))
        } catch {
            sendData(Data("error".utf8))
        }
    }


}
