import AppKit
import Darwin
import Foundation

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private var statusItem: NSStatusItem!
    private let menu = NSMenu()
    private let stateItem = NSMenuItem(title: "Stringcast: Running", action: nil, keyEquivalent: "")
    private let runtimePid = Int32(ProcessInfo.processInfo.environment["STRINGCAST_APP_RUNTIME_PID"] ?? "")

    private var executableURL: URL {
        Bundle.main.bundleURL
            .appendingPathComponent("Contents")
            .appendingPathComponent("MacOS")
            .appendingPathComponent("Stringcast")
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        configureStatusItem()
    }

    private func configureStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.title = "Stringcast On"
        statusItem.menu = menu

        menu.delegate = self
        stateItem.isEnabled = false
        menu.addItem(stateItem)
        menu.addItem(NSMenuItem.separator())

        addMenuItem("Status", #selector(showStatus), "i")
        addMenuItem("Request Permissions", #selector(requestPermissions), "r")
        addMenuItem("Run API Test", #selector(runApiTest), "t")
        addMenuItem("Open Config", #selector(openConfig), "o")
        addMenuItem("Open Logs", #selector(openLogs), "l")
        addMenuItem("Reveal App Executable", #selector(revealAppExecutable), "h")

        menu.addItem(NSMenuItem.separator())
        addMenuItem("Quit", #selector(quit), "q")
        updateMenuState()
    }

    func menuWillOpen(_ menu: NSMenu) {
        updateMenuState()
    }

    private func addMenuItem(_ title: String, _ action: Selector, _ keyEquivalent: String = "") {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: keyEquivalent)
        item.target = self
        menu.addItem(item)
    }

    @objc private func showStatus() {
        runCommand(title: "Stringcast Status", arguments: ["status"])
    }

    @objc private func requestPermissions() {
        DispatchQueue.global(qos: .userInitiated).async {
            let result = self.commandOutput(arguments: ["request-permissions"])
            DispatchQueue.main.async {
                self.showPermissionAlert(message: result.output)
            }
        }
    }

    @objc private func runApiTest() {
        runCommand(title: "Stringcast API Test", arguments: ["api-test"])
    }

    @objc private func openConfig() {
        DispatchQueue.global(qos: .userInitiated).async {
            let result = self.commandOutput(arguments: ["show-config"])
            DispatchQueue.main.async {
                guard result.exitCode == 0 else {
                    self.showAlert(title: "Could Not Find Config", message: result.output)
                    return
                }

                let path = result.output.trimmingCharacters(in: .whitespacesAndNewlines)
                NSWorkspace.shared.open(URL(fileURLWithPath: path))
            }
        }
    }

    @objc private func openLogs() {
        NSWorkspace.shared.open(logDirectoryURL())
    }

    @objc private func revealAppExecutable() {
        NSWorkspace.shared.activateFileViewerSelecting([executableURL])
    }

    @objc private func quit() {
        if let runtimePid, runtimePid > 0 {
            Darwin.kill(runtimePid, SIGTERM)
        }
        NSApp.terminate(nil)
    }

    private func runCommand(title: String, arguments: [String]) {
        DispatchQueue.global(qos: .userInitiated).async {
            let result = self.commandOutput(arguments: arguments)
            DispatchQueue.main.async {
                self.showAlert(title: title, message: result.output, isError: result.exitCode != 0)
            }
        }
    }

    private func commandOutput(arguments: [String]) -> (exitCode: Int32, output: String) {
        let process = Process()
        process.executableURL = executableURL
        process.arguments = arguments

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        do {
            try process.run()
            process.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
            return (process.terminationStatus, output?.isEmpty == false ? output! : "(no output)")
        } catch {
            return (1, error.localizedDescription)
        }
    }

    private func updateMenuState() {
        let running = runtimeIsRunning()
        stateItem.title = running ? "Stringcast: Running" : "Stringcast: Stopped"
        statusItem.button?.title = running ? "Stringcast On" : "Stringcast Off"
    }

    private func runtimeIsRunning() -> Bool {
        guard let runtimePid, runtimePid > 0 else {
            return false
        }
        return Darwin.kill(runtimePid, 0) == 0
    }

    private func logDirectoryURL() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let directory = base.appendingPathComponent("Stringcast", isDirectory: true)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }

    private func showAlert(title: String, message: String, isError: Bool = false) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = isError ? .warning : .informational
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    private func showPermissionAlert(message: String) {
        let alert = NSAlert()
        alert.messageText = "Stringcast Needs Permissions"
        alert.informativeText = """
        \(message)

        Grant permissions to Stringcast.app. The runtime now runs as the app executable:
        \(executableURL.path)
        """
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Open Accessibility")
        alert.addButton(withTitle: "Open Input Monitoring")
        alert.addButton(withTitle: "Reveal App Executable")
        alert.addButton(withTitle: "OK")

        let response = alert.runModal()
        if response == .alertFirstButtonReturn {
            openSystemSettings("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        } else if response == .alertSecondButtonReturn {
            openSystemSettings("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        } else if response == .alertThirdButtonReturn {
            revealAppExecutable()
        }
    }

    private func openSystemSettings(_ urlString: String) {
        guard let url = URL(string: urlString) else {
            return
        }
        NSWorkspace.shared.open(url)
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
