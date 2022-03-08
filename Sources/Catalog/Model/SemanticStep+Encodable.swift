import Foundation
import CookInSwift

extension Step: Encodable {
    enum CodingKeys: String, CodingKey {
        case description
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        try container.encode(directions.map{ $0.description }.joined(), forKey: .description)
    }
}
