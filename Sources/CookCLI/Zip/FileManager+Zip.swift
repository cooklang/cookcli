//
//  File.swift
//  
//
//  Created by Alexey Dubovskoy on 09/08/2021.
//

import Foundation
import ZIPFoundation

extension FileManager {

    /// Unzips the contents at the specified source URL to the destination URL.
    ///
    /// - Parameters:
    ///   - sourceURL: The file URL pointing to an existing ZIP file.
    ///   - destinationURL: The file URL that identifies the destination directory of the unzip operation.
    ///   - skipCRC32: Optional flag to skip calculation of the CRC32 checksum to improve performance.
    ///   - progress: A progress object that can be used to track or cancel the unzip operation.
    ///   - preferredEncoding: Encoding for entry paths. Overrides the encoding specified in the archive.
    /// - Throws: Throws an error if the source item does not exist or the destination URL is not writable.
    public func unzipArchive(archive: Archive, to destinationURL: URL, skipCRC32: Bool = false,
                             preferredEncoding: String.Encoding? = nil) throws {

        // Defer extraction of symlinks until all files & directories have been created.
        // This is necessary because we can't create links to files that haven't been created yet.
        let sortedEntries = archive.sorted { (left, right) -> Bool in
            switch (left.type, right.type) {
            case (.directory, .file): return true
            case (.directory, .symlink): return true
            case (.file, .symlink): return true
            default: return false
            }
        }        

        for entry in sortedEntries {
            let path = preferredEncoding == nil ? entry.path : entry.path(using: preferredEncoding!)
            let destinationEntryURL = destinationURL.appendingPathComponent(path)
            guard destinationEntryURL.isContained(in: destinationURL) else {
                throw CocoaError(.fileReadInvalidFileName,
                                 userInfo: [NSFilePathErrorKey: destinationEntryURL.path])
            }

            try archive.extract(entry, to: destinationEntryURL, skipCRC32: skipCRC32)
        }
    }
}
