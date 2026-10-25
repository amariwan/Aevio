import SwiftUI

struct ContentView: View {
    @StateObject private var client = AevioClient()
    @State private var showingSettings = false

    var body: some View {
        NavigationStack {
            Group {
                if client.pairing == nil {
                    PairingView(client: client)
                } else if let state = client.state {
                    DashboardView(client: client, state: state)
                } else if let error = client.error {
                    ContentUnavailableView {
                        Label("Couldn't connect", systemImage: "wifi.exclamationmark")
                    } description: {
                        Text(error)
                    } actions: {
                        Button("Try again") { Task { await client.refresh() } }
                            .buttonStyle(.borderedProminent)
                    }
                } else {
                    ProgressView("Connecting to Aevio…")
                }
            }
            .navigationTitle("Aevio")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { showingSettings = true } label: { Image(systemName: "gearshape") }
                        .accessibilityLabel("Connection settings")
                }
                if client.isLoading {
                    ToolbarItem(placement: .topBarLeading) {
                        ProgressView()
                            .controlSize(.small)
                            .accessibilityLabel("Loading")
                    }
                }
            }
            .sheet(isPresented: $showingSettings) { PairingView(client: client) }
        }
        .task { client.startPolling() }
        .onDisappear { client.stopPolling() }
    }
}

private struct DashboardView: View {
    @ObservedObject var client: AevioClient
    let state: AevioState

    var body: some View {
        List {
            Section {
                Button {
                    Task {
                        if client.sessionIsActive { await client.stopFocusSession() }
                        else { await client.startFocusSession() }
                    }
                } label: {
                    Label(client.sessionIsActive ? "End focus session" : "Start focus session",
                          systemImage: client.sessionIsActive ? "stop.circle.fill" : "target")
                }
                .buttonStyle(.borderedProminent)
                .disabled(client.isLoading)
                .sensoryFeedback(.impact(weight: .light), trigger: client.sessionIsActive)
            }
            Section("Timers") {
                if state.timers.isEmpty {
                    ContentUnavailableView("No timers", systemImage: "timer", description: Text("Create a timer in the Aevio desktop app."))
                } else {
                    ForEach(state.timers) { timer in
                        Button { Task { await client.toggle(timer) } } label: {
                            HStack(spacing: 12) {
                                Image(systemName: timer.isRunning ? "pause.circle.fill" : "play.circle")
                                    .foregroundStyle(timer.isRunning ? .orange : Color.accentColor)
                                    .imageScale(.large)
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(timer.label).lineLimit(1)
                                    Text(timer.isRunning ? "Running" : "Paused")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer()
                                Text(Duration.milliseconds(timer.elapsed(at: state.nowMs)))
                                    .monospacedDigit()
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .disabled(client.isLoading)
                        .accessibilityLabel("\(timer.label), \(timer.isRunning ? "running" : "paused")")
                        .accessibilityHint("Double tap to toggle the timer")
                        .accessibilityValue("Elapsed \(Duration.milliseconds(timer.elapsed(at: state.nowMs)))")
                    }
                }
            }
            if let error = client.error {
                Section { Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.red) }
            }
        }
        .refreshable { await client.refresh() }
    }
}

private struct PairingView: View {
    @ObservedObject var client: AevioClient
    @Environment(\.dismiss) private var dismiss
    @State private var host = ""
    @State private var port = "48731"
    @State private var token = ""
    @FocusState private var focusedField: Field?

    private enum Field { case host, port, token }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text("Use the pairing details from Aevio on your Mac. The connection stays on your local network.")
                        .font(.callout).foregroundStyle(.secondary)
                }
                Section("Aevio on your Mac") {
                    TextField("Mac hostname or IP", text: $host)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .focused($focusedField, equals: .host)
                        .submitLabel(.next)
                    TextField("Port", text: $port).keyboardType(.numberPad)
                        .focused($focusedField, equals: .port)
                    if !port.isEmpty, UInt16(port) == nil {
                        Text("Enter a valid port number (1–65535).")
                            .font(.caption).foregroundStyle(.red)
                    }
                    SecureField("Pairing token", text: $token)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .focused($focusedField, equals: .token)
                        .submitLabel(.done)
                }
                Section {
                    Button("Pair iPhone") {
                        guard let portNumber = UInt16(port), !host.isEmpty, !token.isEmpty else { return }
                        client.savePairing(host: host, port: portNumber, token: token)
                        Task { await client.refresh() }
                        dismiss()
                    }
                    .disabled(host.isEmpty || token.isEmpty || UInt16(port) == nil)
                    if client.pairing != nil {
                        Button("Forget Pairing", role: .destructive) { client.clearPairing(); dismiss() }
                    }
                }
            }
            .navigationTitle("Connect Aevio")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Close") { dismiss() } }
                if focusedField == .port {
                    ToolbarItem(placement: .keyboard) {
                        Button("Done") { focusedField = nil }
                    }
                }
            }
            .onAppear {
                if let pairing = client.pairing { host = pairing.host; port = String(pairing.port); token = pairing.token }
            }
        }
    }
}

private extension Duration {
    static func milliseconds(_ value: UInt64) -> String {
        let totalSeconds = value / 1_000
        return String(format: "%02d:%02d:%02d", totalSeconds / 3_600, (totalSeconds / 60) % 60, totalSeconds % 60)
    }
}