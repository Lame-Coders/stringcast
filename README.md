# ✨ Stringcast (Wayland UI Edition)

Stringcast is a lightweight, floating desktop utility that uses AI to instantly transform your text. 

* It acts as your persistent, "always-on-top" AI assistant right on your desktop.
* It is perfect for fixing grammar, adopting a professional tone, or translating a paragraph.
* This project is specifically designed as a secure, root-free solution for modern Linux desktops running Wayland.

---

## 🌟 Key Features

* **Always-On-Top UI:** A sleek, floating PyQt6 interface that stays accessible over your other windows.
* **Instant AI Transformations:** Powered by Google's `gemini-2.0-flash` (and OpenAI as a fallback) to rewrite, translate, or summarize text instantly.
* **Frictionless Workflow:** Simply paste your text, select a transformation, and copy the polished result back to your clipboard.
* **Standalone Executable:** Fully compiled with PyInstaller—users don't need to install Python or system libraries to run it.

## 🛡️ The Wayland Security Model

If you use a modern Linux desktop (GNOME, KDE on Wayland), you might wonder why this isn't a background script that listens to your keyboard. 

* **Strict Isolation:** By design, Wayland strictly isolates applications. It intentionally blocks global key-logging and synthetic keystroke injection to protect users from malicious software.
* **The Sudo Problem:** While workarounds exist (like reading raw hardware events via `/dev/input`), they require running scripts as `root` (using `sudo`).
* **The Environment Conflict:** Running desktop scripts as `root` is insecure, cumbersome for daily use, and breaks standard Python virtual environments.
* **Our Solution:** This application bypasses the issue entirely using a native graphical interface. It provides a seamless copy-paste workflow that fully respects Wayland's security sandbox—meaning no `sudo` required!

---

## 🚀 Installation & Usage (For Users)

You do not need to install Python or touch the terminal to use Stringcast.

**1. Download the App**
* Navigate to the [Releases page](https://github.com/mclovin22117/stringcast_wayland/releases) of this repository.
* Download the latest `stringcast_ui.zip` file and extract it.

**2. Configure your API Key**
* In the exact same folder where you extracted the `stringcast_ui` app, create a new text file named `.env`.
* Open the file and add your Gemini API key (and optionally OpenAI):
```env
  GEMINI_API_KEY="your_actual_key_here"
  # OPENAI_API_KEY="optional_fallback_key"
  ```

**3. Run the Application**
* **Make it Executable:** Right-click the downloaded `stringcast_ui` file -> **Properties** -> **Permissions** -> check **"Allow executing file as program"**. *(Alternatively, run `chmod +x stringcast_ui` in your terminal).*
* **Launch:** Double-click the file to open the floating window and start transforming text!

---

## 🛠️ Development & Building from Source

Want to add custom prompts, new languages, or UI features? Building Stringcast from source is straightforward.

**1. Clone & Setup**
```bash
git clone [https://github.com/mclovin22117/stringcast_wayland.git](https://github.com/mclovin22117/stringcast_wayland.git)
cd stringcast_wayland
python3 -m venv venv
source venv/bin/activate
```

**2. Install Dependencies**
```bash
pip install PyQt6 google-generativeai openai python-dotenv pyinstaller
```

**3. Run Locally**
* Ensure your `.env` file is in the root directory.
* Run the app directly via Python:
```bash
  python stringcast_ui.py
  ```

**4. Build the Standalone Executable**
To compile the app into a single distributable binary, use the included PyInstaller configuration:
```bash
pyinstaller --noconsole --onefile --hidden-import openai --hidden-import google.generativeai --hidden-import dotenv stringcast_ui.py
```
* Your compiled application will be generated inside the `dist/` folder.

---

## 🤝 Contributing

Contributions are always welcome! Whether it is adding a new AI provider, tweaking the UI, or optimizing the build size:

* Fork the Project
* Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
* Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
* Push to the Branch (`git push origin feature/AmazingFeature`)
* Open a Pull Request

## 📄 License

Distributed under the MIT License. See the `LICENSE` file for more information.