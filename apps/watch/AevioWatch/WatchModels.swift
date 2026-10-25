import Foundation

struct WatchTimer: Codable, Identifiable, Equatable {
    let id: String
    let label: String
    let startedAt: UInt64?
    let accumulatedMs: UInt64

    var isRunning: Bool { startedAt != nil }

    func elapsed(at now: UInt64) -> UInt64 {
        accumulatedMs + (startedAt.map { max(0, now - $0) } ?? 0)
    }
}

struct WatchState: Codable {
    let timers: [WatchTimer]
    let nowMs: UInt64
}

struct WatchPairing: Codable {
    let host: String
    let port: UInt16
    let token: String
}

enum WatchConnectionError: LocalizedError {
    case notPaired
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .notPaired: "Pair this Watch with Aevio first."
        case .invalidResponse: "Aevio returned an invalid response."
        }
    }
}
