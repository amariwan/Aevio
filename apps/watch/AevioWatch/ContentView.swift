import SwiftUI

struct ContentView: View {
    @StateObject private var client = AevioClient()
    @State private var showingPairing = false

    var body: some View {
        NavigationStack {
            Group {
                if client.pairing == nil {
                    PairingView(client: client)
                } else if let state = client.state {
                    TimerListView(client: client, state: state)
                } else {
                    VStack(spacing: 8) {
                        ProgressView("Connecting…")
                        if let error = client.error {
                            Text(error)
                                .font(.caption2)
                                .foregroundStyle(.red)
                                .multilineTextAlignment(.center)
                        }
                    }
                }
            }
            .navigationTitle("Aevio")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { showingPairing = true } label: { Image(systemName: "gearshape") }
                        .accessibilityLabel("Connection settings")
                }
                if client.isLoading {
                    ToolbarItem(placement: .topBarLeading) {
                        ProgressView().controlSize(.mini)
                            .accessibilityLabel("Loading")
                    }
                }
            }
            .sheet(isPresented: $showingPairing) {
                PairingView(client: client)
            }
        }
        .task { client.startPolling() }
        .onDisappear { client.stopPolling() }
    }
}

private struct TimerListView: View {
    @ObservedObject var client: AevioClient
    let state: WatchState

    var body: some View {
        List(state.timers) { timer in
            Button {
                Task { if timer.isRunning { await client.pause(timer) } else { await client.start(timer) } }
            } label: {
                HStack {
                    VStack(alignment: .leading) {
                        Text(timer.label).lineLimit(1)
                        Text(timer.isRunning ? "Running" : "Paused")
                            .font(.caption2)
                            .foregroundStyle(timer.isRunning ? .green : .secondary)
                    }
                    Spacer()
                    Image(systemName: timer.isRunning ? "pause.fill" : "play.fill")
                        .imageScale(.medium)
                }
            }
            .disabled(client.isLoading)
            .accessibilityLabel("\(timer.label), \(timer.isRunning ? "running" : "paused")")
            .accessibilityHint("Double tap to toggle the timer")
            .sensoryFeedback(.impact(weight: .light), trigger: timer.isRunning)
        }
        .overlay {
            if let error = client.error {
                Text(error).font(.caption2).padding()
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

    var body: some View {
        Form {
            Section("Aevio on your Mac") {
                TextField("Mac hostname or IP", text: $host)
                    .textInputAutocapitalization(.never).autocorrectionDisabled()
                TextField("Port", text: $port)
                    .keyboardType(.numberPad)
                if !port.isEmpty, UInt16(port) == nil {
                    Text("Enter a valid port (1–65535).")
                        .font(.caption2).foregroundStyle(.red)
                }
                TextField("Pairing token", text: $token)
                    .textInputAutocapitalization(.never).autocorrectionDisabled()
            }
            Button("Pair Watch") {
                guard let portNumber = UInt16(port), !host.isEmpty else { return }
                client.savePairing(host: host, port: portNumber, token: token)
                Task { await client.refresh() }
                dismiss()
            }
            .disabled(host.isEmpty || UInt16(port) == nil)
            if client.pairing != nil {
                Button("Forget Pairing", role: .destructive) {
                    client.clearPairing()
                    dismiss()
                }
            }
            if let error = client.error {
                Text(error).font(.caption2).foregroundStyle(.red)
            }
        }
        .onAppear {
            if let pairing = client.pairing {
                host = pairing.host
                port = String(pairing.port)
                token = pairing.token
            }
        }
    }
}