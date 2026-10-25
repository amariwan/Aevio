import Foundation

@MainActor
final class AevioClient: ObservableObject {
    @Published private(set) var state: AevioState?
    @Published private(set) var error: String?
    @Published private(set) var isLoading = false
    @Published private(set) var sessionIsActive = false

    private let defaults = UserDefaults.standard
    private let session: URLSession
    private var pollingTask: Task<Void, Never>?

    init() {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 8
        configuration.timeoutIntervalForResource = 12
        session = URLSession(configuration: configuration)
    }

    var pairing: AevioPairing? {
        guard let host = defaults.string(forKey: "aevio.ios.host"),
              let token = defaults.string(forKey: "aevio.ios.token"),
              let port = defaults.object(forKey: "aevio.ios.port") as? Int else { return nil }
        return AevioPairing(host: host, port: UInt16(port), token: token)
    }

    func savePairing(host: String, port: UInt16, token: String) {
        defaults.set(host.trimmingCharacters(in: .whitespacesAndNewlines), forKey: "aevio.ios.host")
        defaults.set(Int(port), forKey: "aevio.ios.port")
        defaults.set(token.trimmingCharacters(in: .whitespacesAndNewlines), forKey: "aevio.ios.token")
        error = nil
    }

    func clearPairing() {
        defaults.removeObject(forKey: "aevio.ios.host")
        defaults.removeObject(forKey: "aevio.ios.port")
        defaults.removeObject(forKey: "aevio.ios.token")
        state = nil
        sessionIsActive = false
    }

    func startPolling() {
        pollingTask?.cancel()
        pollingTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refresh()
                try? await Task.sleep(for: .seconds(15))
            }
        }
    }

    func stopPolling() { pollingTask?.cancel() }

    func refresh() async { await request(path: "/v1/watch/state", method: "GET") }

    func toggle(_ timer: AevioTimer) async {
        await request(path: "/v1/watch/timers/\(timer.id)/\(timer.isRunning ? "pause" : "start")", method: "POST")
    }

    func startFocusSession() async {
        if await request(path: "/v1/watch/session/start", method: "POST", body: Data(#"{"kind":"focus","label":"Focus"}"#.utf8)) {
            sessionIsActive = true
        }
    }

    func stopFocusSession() async {
        if await request(path: "/v1/watch/session/stop", method: "POST") {
            sessionIsActive = false
        }
    }

    @discardableResult
    private func request(path: String, method: String, body: Data? = nil) async -> Bool {
        guard let pairing else {
            error = AevioConnectionError.notPaired.localizedDescription
            return false
        }
        guard let url = URL(string: "http://\(pairing.host):\(pairing.port)\(path)") else {
            error = AevioConnectionError.invalidAddress.localizedDescription
            return false
        }

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body
        request.setValue("Bearer \(pairing.token)", forHTTPHeaderField: "Authorization")
        if body != nil { request.setValue("application/json", forHTTPHeaderField: "Content-Type") }
        isLoading = true
        defer { isLoading = false }
        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
                throw AevioConnectionError.invalidResponse
            }
            state = try JSONDecoder().decode(AevioState.self, from: data)
            error = nil
            return true
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }
}
