import csv
import os
import platform
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


BASE_DIR = Path(__file__).resolve().parent
CSV_PATH = BASE_DIR / "apps_config.csv"


# --- Dependency Management & Bootstrap ---
def install_dependencies():
    """
    Attempt to install PySide6 using the system package manager.
    Fall back to pip if necessary.
    """
    distro_id = ""
    try:
        os_release = Path("/etc/os-release")
        if os_release.exists():
            for line in os_release.read_text(encoding="utf-8").splitlines():
                if line.startswith("ID="):
                    distro_id = line.split("=", 1)[1].strip().strip('"').lower()
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
        cmds = [["sudo", "pacman", "-Sy"], ["sudo", "pacman", "-S", "--noconfirm", "pyside6"]]
    else:
        print("Unknown distro or generic Linux. Attempting pip install...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "PySide6"])
        return

    try:
        for cmd in cmds:
            print(f"Executing: {' '.join(cmd)}")
            subprocess.check_call(cmd)
    except subprocess.CalledProcessError:
        print("System package installation failed. Attempting pip fallback...")
        try:
            subprocess.check_call([sys.executable, "-m", "pip", "install", "PySide6"])
        except Exception:
            print("Failed to install PySide6.")
            sys.exit(1)


try:
    from PySide6.QtCore import QThread, Qt, Signal
    from PySide6.QtWidgets import (
        QApplication,
        QCheckBox,
        QFormLayout,
        QGroupBox,
        QHBoxLayout,
        QInputDialog,
        QLabel,
        QLineEdit,
        QMainWindow,
        QMessageBox,
        QPushButton,
        QScrollArea,
        QTabWidget,
        QTextEdit,
        QVBoxLayout,
        QWidget,
    )
except ImportError:
    print("PySide6 not found. Initiating bootstrap installation...")
    install_dependencies()
    try:
        from PySide6.QtCore import QThread, Qt, Signal
        from PySide6.QtWidgets import (
            QApplication,
            QCheckBox,
            QFormLayout,
            QGroupBox,
            QHBoxLayout,
            QInputDialog,
            QLabel,
            QLineEdit,
            QMainWindow,
            QMessageBox,
            QPushButton,
            QScrollArea,
            QTabWidget,
            QTextEdit,
            QVBoxLayout,
            QWidget,
        )
    except ImportError:
        sys.exit(1)


@dataclass
class AppEntry:
    category: str
    label: str
    package_name: str = ""
    flatpak_id: str = ""
    exec_name: str = ""
    notes: str = ""

    @property
    def is_flatpak(self):
        return bool(self.flatpak_id)

    @property
    def display_name(self):
        suffix = " (flatpak)" if self.is_flatpak else ""
        return f"{self.label}{suffix}"


class WorkerThread(QThread):
    log_signal = Signal(str)
    finished_signal = Signal()

    def __init__(self, tasks, sudo_password=None):
        super().__init__()
        self.tasks = tasks
        self.sudo_password = sudo_password
        self.ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

    def run(self):
        for desc, cmd in self.tasks:
            self.log_signal.emit(f"\n[INFO] Starting: {desc}")

            final_cmd = cmd
            display_cmd = " ".join(cmd)

            if self.sudo_password:
                final_cmd = ["sudo", "-S"] + cmd
                display_cmd = f"sudo -S {' '.join(cmd)}"

            self.log_signal.emit(f"Command: {display_cmd}")

            try:
                process = subprocess.Popen(
                    final_cmd,
                    cwd=str(BASE_DIR),
                    stdin=subprocess.PIPE if self.sudo_password else None,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    bufsize=1,
                )

                if self.sudo_password and process.stdin:
                    try:
                        process.stdin.write(self.sudo_password + "\n")
                        process.stdin.flush()
                    except (BrokenPipeError, OSError):
                        pass

                if process.stdout:
                    for line in process.stdout:
                        clean_line = self.ansi_escape.sub('', line)
                        self.log_signal.emit(clean_line.rstrip())

                process.wait()

                if process.returncode == 0:
                    self.log_signal.emit(f"[SUCCESS] {desc} completed successfully.")
                else:
                    self.log_signal.emit(f"[ERROR] {desc} failed with return code {process.returncode}.")
            except Exception as exc:
                self.log_signal.emit(f"[EXCEPTION] Failed to run {desc}: {exc}")

        self.log_signal.emit("\n[DONE] All tasks finished.")
        self.finished_signal.emit()


class ToolboxWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("The Linux IT Guy Toolbox")
        self.resize(760, 880)

        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QVBoxLayout(central_widget)

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
        self.link_install_remove_checkboxes()

        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.log_output.setStyleSheet("background-color: #1e1e1e; color: #00ff00; font-family: Monospace;")
        self.log_output.setPlaceholderText("Process logs will appear here...")

        log_group = QGroupBox("Process Log")
        log_layout = QVBoxLayout()
        log_layout.addWidget(self.log_output)
        log_group.setLayout(log_layout)
        main_layout.addWidget(log_group, stretch=1)

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
        if not CSV_PATH.exists():
            return categories

        with CSV_PATH.open(newline='', encoding='utf-8') as csvfile:
            reader = csv.DictReader(csvfile)
            field_names = {name.lower(): name for name in (reader.fieldnames or [])}

            for row in reader:
                if not row:
                    continue

                category = (row.get(field_names.get("category", "Category"), "") or "").strip()
                label = (row.get(field_names.get("label", "Label"), "") or "").strip()
                package_name = (row.get(field_names.get("package name", "Package Name"), "") or "").strip()
                flatpak_id = (row.get(field_names.get("flatpak id", "Flatpak ID"), "") or "").strip()
                exec_name = (row.get(field_names.get("exec name", "Exec Name"), "") or "").strip()
                notes = (row.get(field_names.get("notes", "Notes"), "") or "").strip()

                if not label:
                    legacy_label = (row.get(field_names.get("app name", "App Name"), "") or "").strip()
                    legacy_flatpak = (row.get(field_names.get("flatpak location", "Flatpak Location"), "") or "").strip()
                    label = legacy_label
                    package_name = package_name or legacy_label
                    flatpak_id = flatpak_id or legacy_flatpak

                if not category or not label:
                    continue

                entry = AppEntry(
                    category=category,
                    label=label,
                    package_name=package_name,
                    flatpak_id=flatpak_id,
                    exec_name=exec_name,
                    notes=notes,
                )
                categories.setdefault(category, []).append(entry)

        return categories

    def get_admin_categories(self):
        return {
            "Power Management": [
                ("Enable Bluetooth", "enable-bluetooth.sh"),
                ("Disable Bluetooth", "disable-bluetooth.sh"),
                ("TLP (Laptops)", "install-tlp.sh"),
                ("Powertop", "install-powertop.sh"),
            ],
            "System": [
                ("Update System", "update-system.sh"),
                ("nala (rank mirrors) - Debian-based only", "install-nala.sh"),
                ("Stacer", "install-stacer.sh"),
                ("SWAP Fix", "install-swapfix.sh"),
                ("Fastfetch", "install-fastfetch.sh"),
            ],
        }

    def create_tab_content(self, categories, storage_list):
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        content_widget = QWidget()
        content_layout = QVBoxLayout(content_widget)

        for category, items in categories.items():
            group = QGroupBox(category)
            group_layout = QVBoxLayout()
            for entry in items:
                chk = QCheckBox(entry.display_name)
                if entry.notes:
                    chk.setToolTip(entry.notes)
                group_layout.addWidget(chk)
                storage_list.append((chk, entry))
            group.setLayout(group_layout)
            content_layout.addWidget(group)

        content_layout.addStretch()
        scroll.setWidget(content_widget)
        return scroll

    def create_install_tab(self):
        widget = self.create_tab_content(self.apps_data, self.install_checkboxes)
        self.tabs.addTab(widget, "Install")

    def create_remove_tab(self):
        widget = self.create_tab_content(self.apps_data, self.remove_checkboxes)
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
        layout = QFormLayout(info_widget)
        layout.setLabelAlignment(Qt.AlignRight)
        layout.setFormAlignment(Qt.AlignLeft | Qt.AlignTop)
        layout.setSpacing(10)

        def get_distro_name():
            try:
                os_release = Path("/etc/os-release")
                if os_release.exists():
                    data = {}
                    for line in os_release.read_text(encoding="utf-8").splitlines():
                        if "=" in line:
                            key, value = line.split("=", 1)
                            data[key] = value.strip().strip('"')
                    return data.get("PRETTY_NAME") or data.get("NAME") or "Linux"
            except Exception:
                pass
            return "Linux"

        def get_cpu_info():
            try:
                cpuinfo = Path("/proc/cpuinfo")
                if cpuinfo.exists():
                    for line in cpuinfo.read_text(encoding="utf-8").splitlines():
                        if "model name" in line:
                            return line.split(":", 1)[1].strip()
            except Exception:
                pass
            return platform.processor() or "Unknown"

        def get_uptime():
            try:
                uptime = Path("/proc/uptime")
                if uptime.exists():
                    uptime_seconds = float(uptime.read_text(encoding="utf-8").split()[0])
                    days = int(uptime_seconds // (24 * 3600))
                    hours = int((uptime_seconds % (24 * 3600)) // 3600)
                    minutes = int((uptime_seconds % 3600) // 60)
                    parts = []
                    if days > 0:
                        parts.append(f"{days}d")
                    if hours > 0:
                        parts.append(f"{hours}h")
                    parts.append(f"{minutes}m")
                    return " ".join(parts)
            except Exception:
                pass
            return "Unknown"

        def add_row(label, value):
            value_widget = QLabel(str(value))
            value_widget.setTextInteractionFlags(Qt.TextSelectableByMouse)
            key_label = QLabel(f"<b>{label}:</b>")
            layout.addRow(key_label, value_widget)

        add_row("OS", get_distro_name())
        add_row("Host", platform.node())
        add_row("Kernel", platform.release())
        add_row("Uptime", get_uptime())
        add_row("Shell", os.environ.get("SHELL", "Unknown"))
        add_row("DE/WM", os.environ.get("XDG_CURRENT_DESKTOP", os.environ.get("DESKTOP_SESSION", "Unknown")))
        add_row("CPU", get_cpu_info())
        add_row("Python", sys.version.split()[0])

        self.tabs.addTab(info_widget, "System Info")

    def link_install_remove_checkboxes(self):
        install_map = {entry.label: chk for chk, entry in self.install_checkboxes}

        for chk_remove, entry in self.remove_checkboxes:
            if entry.label in install_map:
                chk_install = install_map[entry.label]
                chk_install.toggled.connect(lambda checked, other=chk_remove: self.sync_checkboxes(checked, other))
                chk_remove.toggled.connect(lambda checked, other=chk_install: self.sync_checkboxes(checked, other))

    def sync_checkboxes(self, checked, other_checkbox):
        if checked:
            other_checkbox.blockSignals(True)
            other_checkbox.setChecked(False)
            other_checkbox.blockSignals(False)

    def on_run_clicked(self):
        tasks = []

        for chk, entry in self.install_checkboxes:
            if chk.isChecked():
                cmd = [
                    "bash",
                    str(BASE_DIR / "main.sh"),
                    "--label",
                    entry.label,
                    "--package",
                    entry.package_name,
                    "--flatpak",
                    entry.flatpak_id,
                    "--exec",
                    entry.exec_name,
                    "install",
                ]
                tasks.append((f"Installing {entry.label}", cmd))

        for chk, entry in self.remove_checkboxes:
            if chk.isChecked():
                cmd = [
                    "bash",
                    str(BASE_DIR / "main.sh"),
                    "--label",
                    entry.label,
                    "--package",
                    entry.package_name,
                    "--flatpak",
                    entry.flatpak_id,
                    "--exec",
                    entry.exec_name,
                    "remove",
                ]
                tasks.append((f"Removing {entry.label}", cmd))

        for chk, script_name in self.admin_checkboxes:
            if chk.isChecked():
                cmd = ["bash", str(BASE_DIR / script_name)]
                tasks.append((f"Running {script_name}", cmd))

        if not tasks:
            QMessageBox.information(self, "No Tasks", "Please select at least one task.")
            return

        pwd, ok = QInputDialog.getText(
            self,
            "Sudo Authentication",
            "Enter your sudo password to proceed:\n(This is required for installations and system tasks)",
            QLineEdit.Password,
        )
        if not ok:
            return

        self.run_btn.setEnabled(False)
        self.log_output.clear()

        self.worker = WorkerThread(tasks, sudo_password=pwd)
        self.worker.log_signal.connect(self.append_log)
        self.worker.finished_signal.connect(self.on_tasks_finished)
        self.worker.start()

    def append_log(self, text):
        self.log_output.append(text)
        scrollbar = self.log_output.verticalScrollBar()
        scrollbar.setValue(scrollbar.maximum())

    def on_tasks_finished(self):
        self.run_btn.setEnabled(True)
        QMessageBox.information(self, "Completed", "All selected tasks executed.")


if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = ToolboxWindow()
    window.show()
    sys.exit(app.exec())
