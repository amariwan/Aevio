import Foundation

@MainActor
final class AevioClient: ObservableObject {
    @Published private(set) var state: WatchState?
    @Published private(set) var error: String?
    @Published private(set) var isLoading = false

    private let defaults = UserDefaults.standard
    private let session: URLSession
    private var timer: Task<Void, Never>?

    init() {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 8
        configuration.timeoutIntervalForResource = 12
        session = URLSession(configuration: configuration)
    }

    var pairing: WatchPairing? {
        guard let host = defaults.string(forKey: "aevio.watch.host"),
              let token = defaults.string(forKey: "aevio.watch.token"),
              let port = defaults.object(forKey: "aevio.watch.port") as? Int else { return nil }
        return WatchPairing(host: host, port: UInt16(port), token: token)
    }

    func savePairing(host: String, port: UInt16, token: String) {
        defaults.set(host.trimmingCharacters(in: .whitespacesAndNewlines), forKey: "aevio.watch.host")
        defaults.set(Int(port), forKey: "aevio.watch.port")
        defaults.set(token.trimmingCharacters(in: .whitespacesAndNewlines), forKey: "aevio.watch.token")
        error = nil
    }

    func clearPairing() {
        defaults.removeObject(forKey: "aevio.watch.host")
        defaults.removeObject(forKey: "aevio.watch.port")
        defaults.removeObject(forKey: "aevio.watch.token")
        state = nil
    }

    func startPolling() {
        timer?.cancel()
        timer = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refresh()
                try? await Task.sleep(for: .seconds(15))
            }
        }
    }

    func stopPolling() { timer?.cancel() }

    func refresh() async {
        await request(path: "/v1/watch/state", method: "GET")
    }

    func start(_ timer: WatchTimer) async {
        await request(path: "/v1/watch/timers/\(timer.id)/start", method: "POST")
    }

    func pause(_ timer: WatchTimer) async {
        await request(path: "/v1/watch/timers/\(timer.id)/pause", method: "POST")
    }

    private func request(path: String, method: String) async {
        guard let pairing else {
            error = WatchConnectionError.notPaired.localizedDescription
            return
        }
        guard let url = URL(string: "http://\(pairing.host):\(pairing.port)\(path)") else {
            error = "The Aevio address is invalid."
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(pairing.token)", forHTTPHeaderField: "Authorization")
        isLoading = true
        defer { isLoading = false }
        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
                throw WatchConnectionError.invalidResponse
            }
            state = try JSONDecoder().decode(WatchState.self, from: data)
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }
}
