import sys
import os
import subprocess
import csv
import platform
import shutil
import threading
import time
import re

# --- Dependency Management & Bootstrap ---
def install_dependencies():
    """
    Attempts to install PySide6 using the system package manager.
    Fallbacks to pip if necessary.
    """
    distro_id = ""
    try:
        if os.path.exists("/etc/os-release"):
            with open("/etc/os-release") as f:
                for line in f:
                    if line.startswith("ID="):
                        distro_id = line.strip().split("=")[1].strip('"').lower()
                        break
    except Exception:
        pass

    print(f"Detected distribution ID: {distro_id}")
    
    cmds = []
    if distro_id in ["debian", "ubuntu", "linuxmint", "pop", "kali"]:
        cmds = [["sudo", "apt", "update"], ["sudo", "apt", "install", "-y", "python3-pyside6"]]
    elif distro_id in ["fedora", "rhel", "centos", "nobara"]:
        cmds = [["sudo", "dnf", "install", "-y", "python3-pyside6"]]
    elif distro_id in ["arch", "manjaro", "endeavouros"]:
        # Arch usually requires a refresh
        cmds = [["sudo", "pacman", "-Sy"], ["sudo", "pacman", "-S", "--noconfirm", "pyside6"]]
    else:
        print("Unknown distro or generic Linux. Attempting pip install...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "PySide6"])
        return

    try:
        for cmd in cmds:
            print(f"Executing: {' '.join(cmd)}")
            # Inherit stdin/stdout for password prompts during bootstrap
            subprocess.check_call(cmd) 
    except subprocess.CalledProcessError:
        print("System package installation failed. Attempting pip fallback...")
        try:
           subprocess.check_call([sys.executable, "-m", "pip", "install", "PySide6"])
        except:
           print("Failed to install PySide6.")
           sys.exit(1)

try:
    from PySide6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, 
                                   QHBoxLayout, QTabWidget, QCheckBox, QScrollArea, 
                                   QPushButton, QLabel, QFrame, QTextEdit, QMessageBox,
                                   QGroupBox, QInputDialog, QLineEdit, QDialog)
    from PySide6.QtCore import Qt, QThread, Signal
    from PySide6.QtGui import QFont, QIcon
except ImportError:
    print("PySide6 not found. Initiating bootstrap installation...")
    install_dependencies()
    try:
        from PySide6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, 
                                       QHBoxLayout, QTabWidget, QCheckBox, QScrollArea, 
                                       QPushButton, QLabel, QFrame, QTextEdit, QMessageBox,
                                       QGroupBox, QInputDialog, QLineEdit, QDialog)
        from PySide6.QtCore import Qt, QThread, Signal
        from PySide6.QtGui import QFont, QIcon
    except ImportError:
        sys.exit(1)

# --- Logic & UI ---

class WorkerThread(QThread):
    log_signal = Signal(str)
    finished_signal = Signal()

    def __init__(self, tasks, sudo_password=None):
        super().__init__()
        self.tasks = tasks # List of (description, command_list)
        self.sudo_password = sudo_password
        # Regex to strip ANSI escape codes (colors)
        self.ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

    def run(self):
        for desc, cmd in self.tasks:
            self.log_signal.emit(f"\n[INFO] Starting: {desc}")
            
            # If we have a password, we run with sudo -S to handle prompts
            # We wrap the command: sudo -S bash -c '...'
            # But cmd is a list like ['bash', 'main.sh', ...]
            # So we prepend sudo -S
            
            final_cmd = cmd
            input_bytes = None
            
            if self.sudo_password:
                final_cmd = ["sudo", "-S"] + cmd
                input_bytes = (self.sudo_password + "\n").encode()
                # Mask password in log
                display_cmd = f"sudo -S {' '.join(cmd)}"
            else:
                display_cmd = ' '.join(cmd)

            self.log_signal.emit(f"Command: {display_cmd}")
            
            try:
                process = subprocess.Popen(
                    final_cmd, 
                    stdin=subprocess.PIPE if self.sudo_password else None,
                    stdout=subprocess.PIPE, 
                    stderr=subprocess.STDOUT, 
                    text=True,
                    bufsize=1  # Line buffered
                )
                
                # If using sudo -S, we must write the password to stdin
                # AND we must do it carefully to avoid deadlock if buffer fills?
                # But here we write once.
                if self.sudo_password and process.stdin:
                    try:
                        process.stdin.write(self.sudo_password + "\n")
                        process.stdin.flush()
                        # process.stdin.close() # Don't close immediately if script reads more?
                        # sudo -S reads one pwd then runs command. 
                        # The command might want stdin too? 
                        # If we assume main.sh doesn't need interactive input, we can close stdin?
                        # Let's keep it open or just let it float.
                    except (BrokenPipeError, OSError):
                        pass

                # Read output
                for line in process.stdout:
                    # Strip ANSI codes
                    clean_line = self.ansi_escape.sub('', line)
                    self.log_signal.emit(clean_line.strip())
                
                process.wait()
                
                if process.returncode == 0:
                    self.log_signal.emit(f"[SUCCESS] {desc} completed successfully.")
                else:
                    self.log_signal.emit(f"[ERROR] {desc} failed with return code {process.returncode}.")
            except Exception as e:
                self.log_signal.emit(f"[EXCEPTION] Failed to run {desc}: {e}")
        
        self.log_signal.emit("\n[DONE] All tasks finished.")
        self.finished_signal.emit()

class ToolboxWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("The Linux IT Guy Toolbox")
        self.resize(700, 850)
        
        # Main Layout
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QVBoxLayout(central_widget)

        # Notebook (Tab Widget)
        self.tabs = QTabWidget()
        main_layout.addWidget(self.tabs)

        self.install_checkboxes = []
        self.remove_checkboxes = []
        self.admin_checkboxes = []
        self.apps_data = self.load_apps_config()
        self.admin_data = self.get_admin_categories()

        self.create_install_tab()
        self.create_remove_tab()
        self.create_admin_tab()
        self.create_system_info_tab()
        
        # Link Install and Remove checkboxes for mutual exclusivity
        self.link_install_remove_checkboxes()
        
        # Log Output
        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.log_output.setStyleSheet("background-color: #1e1e1e; color: #00ff00; font-family: Monospace;")
        self.log_output.setPlaceholderText("Process logs will appear here...")
        
        log_group = QGroupBox("Process Log")
        log_layout = QVBoxLayout()
        log_layout.addWidget(self.log_output)
        log_group.setLayout(log_layout)
        
        main_layout.addWidget(log_group, stretch=1)

        # Buttons
        button_layout = QHBoxLayout()
        
        self.run_btn = QPushButton("Run Selected Tasks")
        self.run_btn.setFixedHeight(40)
        self.run_btn.clicked.connect(self.on_run_clicked)
        button_layout.addWidget(self.run_btn)

        self.quit_btn = QPushButton("Quit")
        self.quit_btn.setFixedHeight(40)
        self.quit_btn.clicked.connect(self.close)
        button_layout.addWidget(self.quit_btn)

        main_layout.addLayout(button_layout)

    def load_apps_config(self):
        categories = {}
        if os.path.exists('apps_config.csv'):
            with open('apps_config.csv', newline='') as csvfile:
                reader = csv.reader(csvfile)
                try:
                    next(reader)
                except StopIteration:
                    pass
                for row in reader:
                    if len(row) >= 3:
                        category, app_name, flatpak_location = row
                        if category not in categories:
                            categories[category] = []
                        categories[category].append((app_name, flatpak_location))
        return categories

    def get_admin_categories(self):
        return {
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

    def create_tab_content(self, categories, storage_list, action_type):
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        content_widget = QWidget()
        content_layout = QVBoxLayout(content_widget)

        for category, items in categories.items():
            group = QGroupBox(category)
            group_layout = QVBoxLayout()
            for app_name, extra in items:
                label_text = app_name
                if action_type in ["install", "remove"] and extra:
                     label_text += " (flatpak)"
                chk = QCheckBox(label_text)
                group_layout.addWidget(chk)
                storage_list.append((chk, app_name, extra))
            group.setLayout(group_layout)
            content_layout.addWidget(group)

        content_layout.addStretch()
        scroll.setWidget(content_widget)
        return scroll

    def create_install_tab(self):
        widget = self.create_tab_content(self.apps_data, self.install_checkboxes, "install")
        self.tabs.addTab(widget, "Install")

    def create_remove_tab(self):
        widget = self.create_tab_content(self.apps_data, self.remove_checkboxes, "remove")
        self.tabs.addTab(widget, "Remove")

    def create_admin_tab(self):
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        content_widget = QWidget()
        content_layout = QVBoxLayout(content_widget)
        for category, options in self.admin_data.items():
            group = QGroupBox(category)
            group_layout = QVBoxLayout()
            for option_name, script_name in options:
                chk = QCheckBox(option_name)
                group_layout.addWidget(chk)
                self.admin_checkboxes.append((chk, script_name))
            group.setLayout(group_layout)
            content_layout.addWidget(group)
        content_layout.addStretch()
        scroll.setWidget(content_widget)
        self.tabs.addTab(scroll, "Administration")

    def create_system_info_tab(self):
        info_widget = QWidget()
        from PySide6.QtWidgets import QFormLayout
        layout = QFormLayout(info_widget)
        layout.setLabelAlignment(Qt.AlignRight)
        layout.setFormAlignment(Qt.AlignLeft | Qt.AlignTop)
        layout.setSpacing(10)

        # Helper to get info safely
        def get_distro_name():
            try:
                if os.path.exists("/etc/os-release"):
                    with open("/etc/os-release") as f:
                        data = {}
                        for line in f:
                            if "=" in line:
                                k, v = line.strip().split("=", 1)
                                data[k] = v.strip('"')
                        return data.get("PRETTY_NAME") or data.get("NAME") or "Linux"
            except: pass
            return "Linux"

        def get_cpu_info():
            try:
                with open("/proc/cpuinfo") as f:
                    for line in f:
                        if "model name" in line:
                            return line.split(":", 1)[1].strip()
            except: pass
            return platform.processor()

        def get_memory_info():
            try:
                mem_total = 0
                mem_avail = 0
                with open("/proc/meminfo") as f:
                    for line in f:
                        if "MemTotal" in line:
                            parts = line.split()
                            mem_total = int(parts[1]) / (1024 * 1024) # GB
                        elif "MemAvailable" in line:
                            parts = line.split()
                            mem_avail = int(parts[1]) / (1024 * 1024) # GB
                if mem_total > 0:
                    return f"{mem_avail:.2f} GB / {mem_total:.2f} GB"
            except: pass
            return "Unknown"

        def get_uptime():
            try:
                with open("/proc/uptime") as f:
                    uptime_seconds = float(f.readline().split()[0])
                    days = int(uptime_seconds // (24 * 3600))
                    hours = int((uptime_seconds % (24 * 3600)) // 3600)
                    minutes = int((uptime_seconds % 3600) // 60)
                    parts = []
                    if days > 0: parts.append(f"{days}d")
                    if hours > 0: parts.append(f"{hours}h")
                    parts.append(f"{minutes}m")
                    return " ".join(parts)
            except: pass
            return "Unknown"

        def add_row(label, value):
            lbl_widget = QLabel(str(value))
            lbl_widget.setTextInteractionFlags(Qt.TextSelectableByMouse)
            # Make the label bold
            key_label = QLabel(f"<b>{label}:</b>")
            layout.addRow(key_label, lbl_widget)

        # Populate Data
        add_row("OS", get_distro_name())
        add_row("Host", platform.node())
        add_row("Kernel", platform.release())
        add_row("Uptime", get_uptime())
        add_row("Shell", os.environ.get("SHELL", "Unknown"))
        add_row("DE/WM", os.environ.get("XDG_CURRENT_DESKTOP", os.environ.get("DESKTOP_SESSION", "Unknown")))
        add_row("CPU", get_cpu_info())
        #add_row("Memory", get_memory_info())
        add_row("Python", sys.version.split()[0])

        self.tabs.addTab(info_widget, "System Info")

    def link_install_remove_checkboxes(self):
        """
        Links Install and Remove checkboxes so that they are mutually exclusive.
        If an app is selected for Install, it is deselected for Remove, and vice versa.
        """
        # Create a map of app_name -> checkbox for the "Install" list
        install_map = {app_name: chk for chk, app_name, _ in self.install_checkboxes}

        for chk_remove, app_name, _ in self.remove_checkboxes:
            if app_name in install_map:
                chk_install = install_map[app_name]
                
                # Connect signals with partial application to capture the specific 'other' checkbox
                # We use a lambda that defaults 'other' to the partner checkbox
                chk_install.toggled.connect(lambda checked, other=chk_remove: self.sync_checkboxes(checked, other))
                chk_remove.toggled.connect(lambda checked, other=chk_install: self.sync_checkboxes(checked, other))

    def sync_checkboxes(self, checked, other_checkbox):
        if checked:
            other_checkbox.blockSignals(True)
            other_checkbox.setChecked(False)
            other_checkbox.blockSignals(False)

    def on_run_clicked(self):
        tasks = []
        for chk, app_name, flatpak_loc in self.install_checkboxes:
            if chk.isChecked():
                cmd = ["bash", "main.sh", app_name, flatpak_loc if flatpak_loc else "", "install"]
                tasks.append((f"Installing {app_name}", cmd))
        for chk, app_name, flatpak_loc in self.remove_checkboxes:
            if chk.isChecked():
                cmd = ["bash", "main.sh", app_name, flatpak_loc if flatpak_loc else "", "remove"]
                tasks.append((f"Removing {app_name}", cmd))
        for chk, script_name in self.admin_checkboxes:
            if chk.isChecked():
                cmd = ["bash", script_name]
                tasks.append((f"Running {script_name}", cmd))

        if not tasks:
            QMessageBox.information(self, "No Tasks", "Please select at least one task.")
            return

        # Prompt for Sudo Password
        pwd, ok = QInputDialog.getText(self, "Sudo Authentication", 
                                       "Enter your sudo password to proceed:\n(This is required for installations)", 
                                       QLineEdit.Password)
        if not ok:
            return # User cancelled

        self.run_btn.setEnabled(False)
        self.log_output.clear()
        
        self.worker = WorkerThread(tasks, sudo_password=pwd)
        self.worker.log_signal.connect(self.append_log)
        self.worker.finished_signal.connect(self.on_tasks_finished)
        self.worker.start()

    def append_log(self, text):
        self.log_output.append(text)
        sb = self.log_output.verticalScrollBar()
        sb.setValue(sb.maximum())

    def on_tasks_finished(self):
        self.run_btn.setEnabled(True)
        QMessageBox.information(self, "Completed", "All selected tasks executed.")

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = ToolboxWindow()
    window.show()
    sys.exit(app.exec())
