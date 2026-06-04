import sys
from PyQt6.QtWidgets import (QApplication, QWidget, QVBoxLayout, QHBoxLayout, 
                             QTextEdit, QPushButton, QComboBox, QLabel)
from PyQt6.QtCore import Qt, QThread, pyqtSignal
from PyQt6.QtGui import QGuiApplication

# Import your working AI engine
from system_enhancer import try_transform_text

class AIWorker(QThread):
    """Runs the API call in the background so the UI doesn't freeze."""
    finished = pyqtSignal(str)

    def __init__(self, text, function_name):
        super().__init__()
        self.text = text
        self.function_name = function_name

    def run(self):
        try:
            result = try_transform_text(self.text, self.function_name)
            self.finished.emit(result)
        except Exception as e:
            self.finished.emit(f"Error: {str(e)}")

class StringcastApp(QWidget):
    def __init__(self):
        super().__init__()
        self.initUI()

    def initUI(self):
        # --- Window Settings ---
        self.setWindowTitle('Stringcast - Hover Mode')
        self.resize(400, 500)
        
        # Make the window float on top of all other applications
        self.setWindowFlags(Qt.WindowType.WindowStaysOnTopHint)
        
        # --- Layout & Widgets ---
        layout = QVBoxLayout()

        # Input Area
        self.input_label = QLabel("Input Text:")
        self.input_box = QTextEdit()
        self.input_box.setPlaceholderText("Type or paste your text here...")
        
        # Options
        self.option_dropdown = QComboBox()
        self.option_dropdown.addItems([
            "Improve Grammar & Flow",
            "Enhance Tone to Professional",
            "Make it More Engaging",
            "Translate to English",
            "Translate to Hindi",
            "Enoji based on this text",
            "Translate to Spanish",
            "Translate to French",
            "Summarize concisely"
        ])
        
        # Transform Button
        self.transform_btn = QPushButton("✨ Transform Text")
        self.transform_btn.setStyleSheet("background-color: #2b5797; color: white; font-weight: bold; padding: 10px;")
        self.transform_btn.clicked.connect(self.run_transformation)

        # Output Area
        self.output_label = QLabel("Enhanced Output:")
        self.output_box = QTextEdit()
        self.output_box.setReadOnly(True)
        self.output_box.setStyleSheet("background-color: #f4f4f4; color: #000000;")

        # Copy Button
        self.copy_btn = QPushButton("📋 Copy to Clipboard")
        self.copy_btn.clicked.connect(self.copy_to_clipboard)

        # Assemble Layout
        layout.addWidget(self.input_label)
        layout.addWidget(self.input_box)
        layout.addWidget(self.option_dropdown)
        layout.addWidget(self.transform_btn)
        layout.addWidget(self.output_label)
        layout.addWidget(self.output_box)
        layout.addWidget(self.copy_btn)

        self.setLayout(layout)

    def run_transformation(self):
        text = self.input_box.toPlainText().strip()
        if not text:
            return

        mode = self.option_dropdown.currentText()
        
        # Update UI state
        self.transform_btn.setText("⏳ Processing...")
        self.transform_btn.setEnabled(False)
        self.output_box.setText("")

        # Run AI in background thread
        self.worker = AIWorker(text, mode)
        self.worker.finished.connect(self.on_transformation_complete)
        self.worker.start()

    def on_transformation_complete(self, result):
        # Restore UI state and show result
        self.output_box.setText(result)
        self.transform_btn.setText("✨ Transform Text")
        self.transform_btn.setEnabled(True)

    def copy_to_clipboard(self):
        clipboard = QGuiApplication.clipboard()
        clipboard.setText(self.output_box.toPlainText())
        self.copy_btn.setText("✅ Copied!")
        
        # Reset button text after 2 seconds
        from PyQt6.QtCore import QTimer
        QTimer.singleShot(2000, lambda: self.copy_btn.setText("📋 Copy to Clipboard"))

if __name__ == '__main__':
    app = QApplication(sys.argv)
    app.setStyle('Fusion') # Clean, modern default look
    ex = StringcastApp()
    ex.show()
    sys.exit(app.exec())