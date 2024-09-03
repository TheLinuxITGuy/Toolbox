import gi
import subprocess
import csv

gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

class OurWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy's Toolbox")
        Gtk.Window.set_default_size(self, 400, 325)
        Gtk.Window.set_position(self, Gtk.WindowPosition.CENTER)
        Gtk.Window.set_resizable(self, False)

        notebook = Gtk.Notebook()
        self.add(notebook)

        # Install tab
        install_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        install_tab.set_border_width(10)
        notebook.append_page(install_tab, Gtk.Label(label="Install"))

        # Remove tab
        remove_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        remove_tab.set_border_width(10)
        notebook.append_page(remove_tab, Gtk.Label(label="Remove"))

        # Administration tab
        admin_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        admin_tab.set_border_width(10)
        notebook.append_page(admin_tab, Gtk.Label(label="Administration"))

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
                ("nala (rank mirrors)", "install-nala.sh"),
                ("Stacer", "install-stacer.sh"),
                ("SWAP Fix", "install-swapfix.sh")
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

        # Run and Quit buttons for Install tab
        hbox_install = Gtk.Box(spacing=6)
        install_tab.pack_start(hbox_install, False, False, 0)

        run_button_install = Gtk.Button(label="Run")
        run_button_install.connect("clicked", self.on_run_button_clicked)
        hbox_install.pack_start(run_button_install, True, True, 0)

        quit_button_install = Gtk.Button(label="Quit")
        quit_button_install.connect("clicked", Gtk.main_quit)
        hbox_install.pack_start(quit_button_install, True, True, 0)

        # Run and Quit buttons for Remove tab
        hbox_remove = Gtk.Box(spacing=6)
        remove_tab.pack_start(hbox_remove, False, False, 0)

        run_button_remove = Gtk.Button(label="Run")
        run_button_remove.connect("clicked", self.on_run_button_clicked)
        hbox_remove.pack_start(run_button_remove, True, True, 0)

        quit_button_remove = Gtk.Button(label="Quit")
        quit_button_remove.connect("clicked", Gtk.main_quit)
        hbox_remove.pack_start(quit_button_remove, True, True, 0)

        # Run and Quit buttons for Administration tab
        hbox_admin = Gtk.Box(spacing=6)
        admin_tab.pack_start(hbox_admin, False, False, 0)

        run_button_admin = Gtk.Button(label="Run")
        run_button_admin.connect("clicked", self.on_run_button_clicked)
        hbox_admin.pack_start(run_button_admin, True, True, 0)

        quit_button_admin = Gtk.Button(label="Quit")
        quit_button_admin.connect("clicked", Gtk.main_quit)
        hbox_admin.pack_start(quit_button_admin, True, True, 0)

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