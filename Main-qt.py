import csv
import subprocess
import sys
import os
import urllib.request  # Added to download get-pip.py
from PyQt5.QtWidgets import QApplication, QMainWindow, QVBoxLayout, QHBoxLayout, QTabWidget, QWidget, QScrollArea, QFrame, QCheckBox, QPushButton, QMessageBox
from PyQt5.QtCore import QThread, pyqtSignal

# Function to install a module using pip
def install_module(module_name):
    subprocess.check_call([sys.executable, "-m", "pip", "install", module_name, "--break-system-packages"])

# Function to check if a command exists
def command_exists(command):
    return subprocess.call(["which", command], stdout=subprocess.PIPE, stderr=subprocess.PIPE) == 0

# Ensure pip is installed
try:
    import pip
except ImportError:
    print("pip is not installed. Installing now...")
    try:
        subprocess.check_call([sys.executable, "-m", "ensurepip", "--upgrade"])
        subprocess.check_call([sys.executable, "-m", "pip", "install", "--upgrade", "pip", "--break-system-packages"])
    except subprocess.CalledProcessError:
        print("ensurepip is not available. Installing pip using get-pip.py...")
        url = "https://bootstrap.pypa.io/get-pip.py"
        get_pip = "get-pip.py"
        urllib.request.urlretrieve(url, get_pip)
        subprocess.check_call([sys.executable, get_pip, "--break-system-packages"])
        subprocess.check_call([sys.executable, "-m", "pip", "install", "--upgrade", "pip", "--break-system-packages"])

# Check if the system is Arch-based
if command_exists("pacman"):
    print("System is Arch-based.")
    # Check if PyQt5 is installed
    if not command_exists("python-pyqt5"):
        print("PyQt5 is not installed. Installing now...")
        subprocess.check_call(["sudo", "pacman", "-S", "--noconfirm", "python-pyqt5"])
else:
    print("System is not Arch-based.")

# Check for the 'PyQt5' module and install it if missing
try:
    from PyQt5.QtWidgets import QApplication, QMainWindow, QVBoxLayout, QHBoxLayout, QTabWidget, QWidget, QScrollArea, QFrame, QCheckBox, QPushButton, QMessageBox
except ImportError:
    print("The 'PyQt5' module is not installed. Installing now...")
    install_module("PyQt5")
    from PyQt5.QtWidgets import QApplication, QMainWindow, QVBoxLayout, QHBoxLayout, QTabWidget, QWidget, QScrollArea, QFrame, QCheckBox, QPushButton, QMessageBox

# Worker thread to run the commands without blocking the GUI
class Worker(QThread):
    progress = pyqtSignal(str)
    finished = pyqtSignal()

    def __init__(self, install_checkboxes, remove_checkboxes, admin_checkboxes):
        super().__init__()
        self.install_checkboxes = install_checkboxes
        self.remove_checkboxes = remove_checkboxes
        self.admin_checkboxes = admin_checkboxes

    def run(self):
        for check_button, app_name, flatpak_location in self.install_checkboxes:
            if check_button.isChecked():
                msg = f"Installing {app_name}..."
                self.progress.emit(msg)
                subprocess.run(["bash", "main.sh", app_name, flatpak_location, "install"])
        for check_button, app_name, flatpak_location in self.remove_checkboxes:
            if check_button.isChecked():
                msg = f"Removing {app_name}..."
                self.progress.emit(msg)
                subprocess.run(["bash", "main.sh", app_name, flatpak_location, "remove"])
        for check_button, script in self.admin_checkboxes:
            if check_button.isChecked():
                msg = f"Running {script}..."
                self.progress.emit(msg)
                subprocess.run(["bash", script])
        self.finished.emit()

class OurWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("The Linux IT Guy Toolbox")
        self.setGeometry(100, 100, 400, 700)
        self.setFixedSize(400, 700)

        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        vbox_main = QVBoxLayout()
        central_widget.setLayout(vbox_main)

        notebook = QTabWidget()
        vbox_main.addWidget(notebook)

        # Create a scrolled window for each tab
        scrolled_window_install = QScrollArea()
        scrolled_window_install.setWidgetResizable(True)
        scrolled_window_remove = QScrollArea()
        scrolled_window_remove.setWidgetResizable(True)
        scrolled_window_admin = QScrollArea()
        scrolled_window_admin.setWidgetResizable(True)

        # Install tab
        install_tab = QWidget()
        install_layout = QVBoxLayout()
        install_tab.setLayout(install_layout)
        scrolled_window_install.setWidget(install_tab)
        notebook.addTab(scrolled_window_install, "Install")

        # Remove tab
        remove_tab = QWidget()
        remove_layout = QVBoxLayout()
        remove_tab.setLayout(remove_layout)
        scrolled_window_remove.setWidget(remove_tab)
        notebook.addTab(scrolled_window_remove, "Remove")

        # Administration tab
        admin_tab = QWidget()
        admin_layout = QVBoxLayout()
        admin_tab.setLayout(admin_layout)
        scrolled_window_admin.setWidget(admin_tab)
        notebook.addTab(scrolled_window_admin, "Administration")

        self.install_checkboxes = []
        self.remove_checkboxes = []
        self.admin_checkboxes = []

        # Read configuration file
        with open('apps_config.csv', newline='') as csvfile:
            reader = csv.reader(csvfile)
            next(reader)  # Skip header row
            categories = {}
            for row in reader:
                category, app_name, flatpak_location = row
                if category not in categories:
                    categories[category] = []
                categories[category].append((app_name, flatpak_location))

        # Create install and remove categories
        for category, apps in categories.items():
            # Install category
            frame_install = QFrame()
            frame_install.setFrameShape(QFrame.StyledPanel)
            install_layout.addWidget(frame_install)
            box_install = QVBoxLayout()
            frame_install.setLayout(box_install)
            for app_name, flatpak_location in apps:
                install_label = app_name + (" (flatpak)" if flatpak_location else "")
                check_button = QCheckBox(install_label)
                box_install.addWidget(check_button)
                check_button.toggled.connect(self.on_check_button_toggled)
                self.install_checkboxes.append((check_button, app_name, flatpak_location))
            # Remove category
            frame_remove = QFrame()
            frame_remove.setFrameShape(QFrame.StyledPanel)
            remove_layout.addWidget(frame_remove)
            box_remove = QVBoxLayout()
            frame_remove.setLayout(box_remove)
            for app_name, flatpak_location in apps:
                remove_label = app_name + (" (flatpak)" if flatpak_location else "")
                check_button = QCheckBox(remove_label)
                box_remove.addWidget(check_button)
                check_button.toggled.connect(self.on_check_button_toggled)
                self.remove_checkboxes.append((check_button, app_name, flatpak_location))

        # Administration categories
        admin_categories = {
            "Power Management": [
                ("Enable Bluetooth", "enable-bluetooth.sh"),
                ("Disable Bluetooth", "disable-bluetooth.sh"),
                ("TLP (Laptops)", "install-tlp.sh"),
                ("Powertop", "install-powertop.sh")
            ],
            "System": [
                ("Update System", "update-system.sh"),
                ("nala (rank mirrors) - Debian-based only", "install-nala.sh"),
                ("Stacer", "install-stacer.sh"),
                ("SWAP Fix", "install-swapfix.sh"),
                ("Fastfetch", "install-fastfetch.sh")
            ]
        }
        for category, options in admin_categories.items():
            frame_admin = QFrame()
            frame_admin.setFrameShape(QFrame.StyledPanel)
            admin_layout.addWidget(frame_admin)
            box_admin = QVBoxLayout()
            frame_admin.setLayout(box_admin)
            for option, script in options:
                check_button = QCheckBox(option)
                box_admin.addWidget(check_button)
                check_button.toggled.connect(self.on_check_button_toggled)
                self.admin_checkboxes.append((check_button, script))

        # Run and Quit buttons
        hbox_buttons = QHBoxLayout()
        vbox_main.addLayout(hbox_buttons)
        run_button = QPushButton("Run")
        run_button.clicked.connect(self.on_run_button_clicked)
        hbox_buttons.addWidget(run_button)
        quit_button = QPushButton("Quit")
        quit_button.clicked.connect(QApplication.instance().quit)
        hbox_buttons.addWidget(quit_button)

    def on_check_button_toggled(self, checked):
        button = self.sender()
        print(f"{button.text()} {'checked' if checked else 'unchecked'}")

    def on_run_button_clicked(self):
        # Disable Run button if needed to prevent multiple clicks
        print("Run button clicked")
        self.worker = Worker(self.install_checkboxes, self.remove_checkboxes, self.admin_checkboxes)
        self.worker.progress.connect(print)
        self.worker.finished.connect(lambda: QMessageBox.information(self, "Done", "Tasks complete."))
        self.worker.start()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = OurWindow()
    window.show()
    sys.exit(app.exec_())