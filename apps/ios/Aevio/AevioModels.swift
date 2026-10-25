import Foundation

struct AevioTimer: Codable, Identifiable, Equatable {
    let id: String
    let label: String
    let startedAt: UInt64?
    let accumulatedMs: UInt64

    var isRunning: Bool { startedAt != nil }

    func elapsed(at now: UInt64) -> UInt64 {
        accumulatedMs + (startedAt.map { now >= $0 ? now - $0 : 0 } ?? 0)
    }
}

struct AevioState: Codable {
    let timers: [AevioTimer]
    let nowMs: UInt64
}

struct AevioPairing {
    let host: String
    let port: UInt16
    let token: String
}

enum AevioConnectionError: LocalizedError {
    case notPaired
    case invalidAddress
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .notPaired: "Connect this iPhone to Aevio on your Mac first."
        case .invalidAddress: "The Aevio Mac address is invalid."
        case .invalidResponse: "Aevio returned an invalid response."
        }
    }
}
