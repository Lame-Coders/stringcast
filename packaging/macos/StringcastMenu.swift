import AppKit
import Foundation

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private let menu = NSMenu()
    private let stateItem = NSMenuItem(title: "Stringcast: Stopped", action: nil, keyEquivalent: "")
    private let startItem = NSMenuItem(title: "Start Stringcast", action: #selector(startRuntime), keyEquivalent: "s")
    private let stopItem = NSMenuItem(title: "Stop Stringcast", action: #selector(stopRuntimeFromMenu), keyEquivalent: "x")
    private var runtimeProcess: Process?
    private var logHandle: FileHandle?

    private var binaryURL: URL {
        Bundle.main.bundleURL
            .appendingPathComponent("Contents")
            .appendingPathComponent("Resources")
            .appendingPathComponent("stringcast")
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        configureStatusItem()
        startRuntime()
    }

    func applicationWillTerminate(_ notification: Notification) {
        stopRuntime()
    }

    private func configureStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.title = "Stringcast"
        statusItem.menu = menu

        stateItem.isEnabled = false
        menu.addItem(stateItem)
        menu.addItem(NSMenuItem.separator())

        startItem.target = self
        menu.addItem(startItem)

        stopItem.target = self
        menu.addItem(stopItem)

        menu.addItem(NSMenuItem.separator())
        addMenuItem("Status", #selector(showStatus), "i")
        addMenuItem("Check Permissions", #selector(showPermissions), "p")
        addMenuItem("Run API Test", #selector(runApiTest), "t")
        addMenuItem("Open Config", #selector(openConfig), "o")
        addMenuItem("Open Logs", #selector(openLogs), "l")

        menu.addItem(NSMenuItem.separator())
        addMenuItem("Quit", #selector(quit), "q")
        updateMenuState()
    }

    private func addMenuItem(_ title: String, _ action: Selector, _ keyEquivalent: String = "") {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: keyEquivalent)
        item.target = self
        menu.addItem(item)
    }

    @objc private func startRuntime() {
        guard runtimeProcess?.isRunning != true else {
            updateMenuState()
            return
        }

        let process = Process()
        process.executableURL = binaryURL
        process.arguments = ["run"]
        process.standardOutput = runtimeLogHandle()
        process.standardError = runtimeLogHandle()
        process.terminationHandler = { [weak self] terminatedProcess in
            DispatchQueue.main.async {
                if self?.runtimeProcess?.processIdentifier == terminatedProcess.processIdentifier {
                    self?.runtimeProcess = nil
                    self?.closeLogHandle()
                    self?.updateMenuState()
                }
            }
        }

        do {
            try process.run()
            runtimeProcess = process
            updateMenuState()
        } catch {
            showAlert(title: "Could Not Start Stringcast", message: error.localizedDescription)
            closeLogHandle()
            updateMenuState()
        }
    }

    @objc private func stopRuntimeFromMenu() {
        stopRuntime()
    }

    private func stopRuntime() {
        guard let process = runtimeProcess else {
            closeLogHandle()
            updateMenuState()
            return
        }

        if process.isRunning {
            process.terminate()
            DispatchQueue.global(qos: .utility).async {
                process.waitUntilExit()
            }
        }

        runtimeProcess = nil
        closeLogHandle()
        updateMenuState()
    }

    @objc private func showStatus() {
        runCommand(title: "Stringcast Status", arguments: ["status"])
    }

    @objc private func showPermissions() {
        runCommand(title: "Stringcast Permissions", arguments: ["check-permissions"])
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

    @objc private func quit() {
        stopRuntime()
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
        process.executableURL = binaryURL
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
        let running = runtimeProcess?.isRunning == true
        stateItem.title = running ? "Stringcast: Running" : "Stringcast: Stopped"
        statusItem.button?.title = running ? "Stringcast On" : "Stringcast Off"
        startItem.isEnabled = !running
        stopItem.isEnabled = running
    }

    private func runtimeLogHandle() -> FileHandle {
        if let logHandle {
            return logHandle
        }

        let url = logFileURL()
        FileManager.default.createFile(atPath: url.path, contents: nil)
        let handle = (try? FileHandle(forWritingTo: url)) ?? FileHandle.standardError
        _ = try? handle.seekToEnd()
        logHandle = handle
        return handle
    }

    private func closeLogHandle() {
        if let logHandle {
            try? logHandle.close()
        }
        logHandle = nil
    }

    private func logDirectoryURL() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let directory = base.appendingPathComponent("Stringcast", isDirectory: true)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }

    private func logFileURL() -> URL {
        logDirectoryURL().appendingPathComponent("stringcast-app.log")
    }

    private func showAlert(title: String, message: String, isError: Bool = false) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = isError ? .warning : .informational
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
