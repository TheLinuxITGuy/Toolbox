import csv
import subprocess
import sys

# Function to install a module using pip
def install_module(module_name):
    subprocess.check_call([sys.executable, "-m", "pip", "install", module_name])

# Function to check if a command exists
def command_exists(command):
    return subprocess.call(["which", command], stdout=subprocess.PIPE, stderr=subprocess.PIPE) == 0

# Check if the system is Arch-based
if command_exists("pacman"):
    print("System is Arch-based.")
    # Check if pip is installed
    if not command_exists("pip"):
        print("pip is not installed. Installing now...")
        subprocess.check_call(["sudo", "pacman", "-S", "--noconfirm", "python-pip"])
    # Check if pygobject is installed
    if not command_exists("python-gi"):
        print("pygobject is not installed. Installing now...")
        subprocess.check_call(["sudo", "pacman", "-S", "--noconfirm", "python-gobject"])
else:
    print("System is not Arch-based.")

# Check for the 'gi' module and install it if missing
try:
    import gi
except ImportError:
    print("The 'gi' module is not installed. Installing now...")
    install_module("pygobject")
    import gi

# Now you can safely use the 'gi' module
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

class OurWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy Toolbox")
        Gtk.Window.set_default_size(self, 400, 700)
        Gtk.Window.set_position(self, Gtk.WindowPosition.CENTER)
        Gtk.Window.set_resizable(self, False)

        vbox_main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.add(vbox_main)

        notebook = Gtk.Notebook()
        vbox_main.pack_start(notebook, True, True, 0)

        # Create a scrolled window for each tab
        scrolled_window_install = Gtk.ScrolledWindow()
        scrolled_window_install.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        scrolled_window_remove = Gtk.ScrolledWindow()
        scrolled_window_remove.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        scrolled_window_admin = Gtk.ScrolledWindow()
        scrolled_window_admin.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)

        # Install tab
        install_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        install_tab.set_border_width(10)
        scrolled_window_install.add(install_tab)
        notebook.append_page(scrolled_window_install, Gtk.Label(label="Install"))

        # Remove tab
        remove_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        remove_tab.set_border_width(10)
        scrolled_window_remove.add(remove_tab)
        notebook.append_page(scrolled_window_remove, Gtk.Label(label="Remove"))

        # Administration tab
        admin_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        admin_tab.set_border_width(10)
        scrolled_window_admin.add(admin_tab)
        notebook.append_page(scrolled_window_admin, Gtk.Label(label="Administration"))

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
            frame_install = Gtk.Frame(label=category)
            install_tab.pack_start(frame_install, True, True, 0)

            box_install = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
            frame_install.add(box_install)

            for app_name, flatpak_location in apps:
                install_label = app_name
                if flatpak_location:
                    install_label += " (flatpak)"
                check_button = Gtk.CheckButton(label=install_label)
                box_install.pack_start(check_button, True, True, 0)
                check_button.connect("toggled", self.on_check_button_toggled)
                self.install_checkboxes.append((check_button, app_name, flatpak_location))

            # Remove category
            frame_remove = Gtk.Frame(label=category)
            remove_tab.pack_start(frame_remove, True, True, 0)

            box_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
            frame_remove.add(box_remove)

            for app_name, flatpak_location in apps:
                remove_label = app_name
                if flatpak_location:
                    remove_label += " (flatpak)"
                check_button = Gtk.CheckButton(label=remove_label)
                box_remove.pack_start(check_button, True, True, 0)
                check_button.connect("toggled", self.on_check_button_toggled)
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
                ("Stacer", "install-stacer.sh"),("SWAP Fix", "install-swapfix.sh"),
                ("Fastfetch", "install-fastfetch.sh")
            ]
        }

        for category, options in admin_categories.items():
            frame_admin = Gtk.Frame(label=category)
            admin_tab.pack_start(frame_admin, True, True, 0)

            box_admin = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
            frame_admin.add(box_admin)

            for option, script in options:
                check_button = Gtk.CheckButton(label=option)
                box_admin.pack_start(check_button, True, True, 0)
                check_button.connect("toggled", self.on_check_button_toggled)
                self.admin_checkboxes.append((check_button, script))

        # Run and Quit buttons
        hbox_buttons = Gtk.Box(spacing=6)
        vbox_main.pack_start(hbox_buttons, False, False, 0)

        run_button = Gtk.Button(label="Run")
        run_button.connect("clicked", self.on_run_button_clicked)
        hbox_buttons.pack_start(run_button, True, True, 0)

        quit_button = Gtk.Button(label="Quit")
        quit_button.connect("clicked", Gtk.main_quit)
        hbox_buttons.pack_start(quit_button, True, True, 0)

    def on_check_button_toggled(self, button):
        if button.get_active():
            print("{} checked".format(button.get_label()))
        else:
            print("{} unchecked".format(button.get_label()))

    def on_run_button_clicked(self, button):
        print("Run button clicked")
        for check_button, app_name, flatpak_location in self.install_checkboxes:
            if check_button.get_active():
                print("Installing {} via main.sh".format(check_button.get_label()))
                subprocess.run(["bash", "main.sh", app_name, flatpak_location, "install"])

        for check_button, app_name, flatpak_location in self.remove_checkboxes:
            if check_button.get_active():
                print("Removing {} via main.sh".format(check_button.get_label()))
                subprocess.run(["bash", "main.sh", app_name, flatpak_location, "remove"])

        for check_button, script in self.admin_checkboxes:
            if check_button.get_active():
                print("Running {} via main.sh".format(check_button.get_label()))
                subprocess.run(["bash", script])

        # Create a message dialog
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=0,
            message_type=Gtk.MessageType.INFO,
            buttons=Gtk.ButtonsType.OK,
            text="Applications have been Installed and/or Removed.",
        )
        dialog.format_secondary_text(
            "Please click the 'Quit' button to exit the application."
        )
        dialog.run()
        dialog.destroy()

window = OurWindow()
window.connect("delete-event", Gtk.main_quit)
window.show_all()
Gtk.main()