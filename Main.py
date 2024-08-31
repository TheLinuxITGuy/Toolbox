#!/usr/bin/python3
# -*- coding: utf-8 -*-

import gi
import subprocess

gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

class OurWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy's Toolbox")
        Gtk.Window.set_default_size(self, 800, 650)  # Doubled the size
        Gtk.Window.set_position(self, Gtk.WindowPosition.CENTER)
        # Prevent the window from being resized
        Gtk.Window.set_resizable(self, False)

        vbox_main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.add(vbox_main)

        # Create a Notebook for tabs
        notebook = Gtk.Notebook()
        vbox_main.pack_start(notebook, True, True, 0)

        # Install tab
        install_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(install_tab, Gtk.Label(label="Install"))

        # Add a scrolled window to the install tab
        scrolled_window_install = Gtk.ScrolledWindow()
        scrolled_window_install.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        install_tab.pack_start(scrolled_window_install, True, True, 0)

        vbox_install = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        scrolled_window_install.add(vbox_install)

        # Development Tools category
        frame_dev_tools = Gtk.Frame(label="Development Tools")
        frame_dev_tools.set_margin_start(10)  # Add margin on the left
        frame_dev_tools.set_margin_end(10)    # Add margin on the right
        vbox_install.pack_start(frame_dev_tools, True, True, 0)

        box_dev_tools = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools.add(box_dev_tools)

        self.dev_tools_checkboxes = []
        dev_tools_options = [("Bottles", "./install-bottles.sh"), ("Boxes", "./install-boxes.sh"),
                             ("Microsoft Visual Studio Code", "./install-code.sh"), ("PyCharm", "./install-pycharm.sh")]

        for option, script in dev_tools_options:
            check_button = Gtk.CheckButton(label=option)
            box_dev_tools.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.dev_tools_checkboxes.append((check_button, script))

        # Communication category
        frame_communication = Gtk.Frame(label="Communication")
        frame_communication.set_margin_start(10)
        frame_communication.set_margin_end(10)
        vbox_install.pack_start(frame_communication, True, True, 0)

        box_communication = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_communication.add(box_communication)

        self.communication_checkboxes = []
        communication_options = [("Discord", "./install-discord.sh"), ("Thunderbird Mail", "./install-thunderbird.sh"), 
                                 ("Slack", "./install-slack.sh")]

        for option, script in communication_options:
            check_button = Gtk.CheckButton(label=option)
            box_communication.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.communication_checkboxes.append((check_button, script))

        # Browsers category
        frame_browsers = Gtk.Frame(label="Browsers")
        frame_browsers.set_margin_start(10)
        frame_browsers.set_margin_end(10)
        vbox_install.pack_start(frame_browsers, True, True, 0)

        box_browsers = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_browsers.add(box_browsers)

        self.browsers_checkboxes = []
        browsers_options = [("Brave Browser", "./install-bravebrowser.sh"), ("Microsoft Edge", "./install-edge.sh"),
                            ("Google Chrome", "./install-chromebrowser.sh"), 
                            ("Firefox", "./install-firefox.sh"), ("Opera", "./install-opera.sh"), 
                            ("Vivaldi", "./install-vivaldi.sh")]

        for option, script in browsers_options:
            check_button = Gtk.CheckButton(label=option)
            box_browsers.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.browsers_checkboxes.append((check_button, script))

        # Multimedia category
        frame_multimedia = Gtk.Frame(label="Multimedia")
        frame_multimedia.set_margin_start(10)
        frame_multimedia.set_margin_end(10)
        vbox_install.pack_start(frame_multimedia, True, True, 0)

        box_multimedia = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_multimedia.add(box_multimedia)

        self.multimedia_checkboxes = []
        multimedia_options = [("GIMP", "./install-gimp.sh"), ("OBS-Studio", "./install-obs.sh"), 
                              ("Audacity", "./install-audacity.sh")]

        for option, script in multimedia_options:
            check_button = Gtk.CheckButton(label=option)
            box_multimedia.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.multimedia_checkboxes.append((check_button, script))

        # Gaming category
        frame_gaming = Gtk.Frame(label="Gaming")
        frame_gaming.set_margin_start(10)
        frame_gaming.set_margin_end(10)
        vbox_install.pack_start(frame_gaming, True, True, 0)

        box_gaming = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_gaming.add(box_gaming)

        self.gaming_checkboxes = []
        gaming_options = [("Lutris", "./install-lutris.sh"), ("Steam & ProtonUp-Qt", "./install-steam&protonupqt.sh")]

        for option, script in gaming_options:
            check_button = Gtk.CheckButton(label=option)
            box_gaming.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.gaming_checkboxes.append((check_button, script))

        # System Utilities category
        frame_system_utils = Gtk.Frame(label="Utilities")
        frame_system_utils.set_margin_start(10)
        frame_system_utils.set_margin_end(10)
        vbox_install.pack_start(frame_system_utils, True, True, 0)

        box_system_utils = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_system_utils.add(box_system_utils)

        self.system_utils_checkboxes = []
        system_utils_options = [("HTop", "./install-htop.sh"), ("Neofetch", "./install-neofetch.sh"), 
                                ("GParted", "./install-gparted.sh"), ("QEMU", "./install-QEMU.sh")]

        for option, script in system_utils_options:
            check_button = Gtk.CheckButton(label=option)
            box_system_utils.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.system_utils_checkboxes.append((check_button, script))

        # Office Tools category
        frame_office_tools = Gtk.Frame(label="Office Tools")
        frame_office_tools.set_margin_start(10)
        frame_office_tools.set_margin_end(10)
        vbox_install.pack_start(frame_office_tools, True, True, 0)

        box_office_tools = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_office_tools.add(box_office_tools)

        self.office_tools_checkboxes = []
        office_tools_options = [("LibreOffice", "./install-libreoffice.sh"), ("OnlyOffice", "./install-onlyoffice.sh")]

        for option, script in office_tools_options:
            check_button = Gtk.CheckButton(label=option)
            box_office_tools.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.office_tools_checkboxes.append((check_button, script))

        """ Miscellaneous category - Future use
        frame_miscellaneous = Gtk.Frame(label="Miscellaneous")
        vbox_install.pack_start(frame_miscellaneous, True, True, 0)

        box_miscellaneous = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_miscellaneous.add(box_miscellaneous)

        self.miscellaneous_checkboxes = []
        miscellaneous_options = [("Any other scripts", "./install-other.sh")]

        for option, script in miscellaneous_options:
            check_button = Gtk.CheckButton(label=option)
            box_miscellaneous.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.miscellaneous_checkboxes.append((check_button, script))
        Miscellaneous category end"""

        # Remove tab
        remove_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(remove_tab, Gtk.Label(label="Remove"))

        # Add a scrolled window to the remove tab
        scrolled_window_remove = Gtk.ScrolledWindow()
        scrolled_window_remove.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        remove_tab.pack_start(scrolled_window_remove, True, True, 0)

        vbox_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        scrolled_window_remove.add(vbox_remove)

        # Development Tools category
        frame_dev_tools_remove = Gtk.Frame(label="Development Tools")
        frame_dev_tools_remove.set_margin_start(10)
        frame_dev_tools_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_dev_tools_remove, True, True, 0)

        box_dev_tools_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools_remove.add(box_dev_tools_remove)

        self.dev_tools_remove_checkboxes = []
        dev_tools_remove_options = [("Bottles", "./remove-bottles.sh"), ("Boxes", "./remove-boxes.sh"),
                                    ("Microsoft Visual Studio Code", "./remove-code.sh"), ("PyCharm", "./remove-pycharm.sh")]

        for option, script in dev_tools_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_dev_tools_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.dev_tools_remove_checkboxes.append((check_button, script))

        # Communication category
        frame_communication_remove = Gtk.Frame(label="Communication")
        frame_communication_remove.set_margin_start(10)
        frame_communication_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_communication_remove, True, True, 0)

        box_communication_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_communication_remove.add(box_communication_remove)

        self.communication_remove_checkboxes = []
        communication_remove_options = [("Discord", "./remove-discord.sh"), ("Thunderbird Mail", "./remove-thunderbird.sh"), 
                                        ("Slack", "./remove-slack.sh")]

        for option, script in communication_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_communication_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.communication_remove_checkboxes.append((check_button, script))

        # Browsers category
        frame_browsers_remove = Gtk.Frame(label="Browsers")
        frame_browsers_remove.set_margin_start(10)
        frame_browsers_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_browsers_remove, True, True, 0)

        box_browsers_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_browsers_remove.add(box_browsers_remove)

        self.browsers_remove_checkboxes = []
        browsers_remove_options = [("Brave Browser", "./remove-bravebrowser.sh"), ("Microsoft Edge", "./remove-edge.sh"),
                                   ("Google Chrome", "./remove-chromebrowser.sh"), 
                                   ("Firefox", "./remove-firefox.sh"), ("Opera", "./remove-opera.sh"), 
                                   ("Vivaldi", "./remove-vivaldi.sh")]

        for option, script in browsers_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_browsers_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.browsers_remove_checkboxes.append((check_button, script))

        # Multimedia category
        frame_multimedia_remove = Gtk.Frame(label="Multimedia")
        frame_multimedia_remove.set_margin_start(10)
        frame_multimedia_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_multimedia_remove, True, True, 0)

        box_multimedia_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_multimedia_remove.add(box_multimedia_remove)

        self.multimedia_remove_checkboxes = []
        multimedia_remove_options = [("GIMP", "./remove-gimp.sh"), ("OBS-Studio", "./remove-obs.sh"), 
                                     ("Audacity", "./remove-audacity.sh")]

        for option, script in multimedia_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_multimedia_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.multimedia_remove_checkboxes.append((check_button, script))

        # Gaming category
        frame_gaming_remove = Gtk.Frame(label="Gaming")
        frame_gaming_remove.set_margin_start(10)
        frame_gaming_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_gaming_remove, True, True, 0)

        box_gaming_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_gaming_remove.add(box_gaming_remove)

        self.gaming_remove_checkboxes = []
        gaming_remove_options = [("Lutris", "./remove-lutris.sh"), ("Steam & ProtonUp-Qt", "./remove-steam&protonupqt.sh")]

        for option, script in gaming_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_gaming_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.gaming_remove_checkboxes.append((check_button, script))

        # System Utilities category
        frame_system_utils_remove = Gtk.Frame(label="Utilities")
        frame_system_utils_remove.set_margin_start(10)
        frame_system_utils_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_system_utils_remove, True, True, 0)

        box_system_utils_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_system_utils_remove.add(box_system_utils_remove)

        self.system_utils_remove_checkboxes = []
        system_utils_remove_options = [("htop", "./remove-htop.sh"), ("Neofetch", "./remove-neofetch.sh"), 
                                       ("GParted", "./remove-gparted.sh"), ("QEMU", "./remove-QEMU.sh")]

        for option, script in system_utils_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_system_utils_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.system_utils_remove_checkboxes.append((check_button, script))

        # Office Tools category
        frame_office_tools_remove = Gtk.Frame(label="Office Tools")
        frame_office_tools_remove.set_margin_start(10)
        frame_office_tools_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_office_tools_remove, True, True, 0)

        box_office_tools_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_office_tools_remove.add(box_office_tools_remove)

        self.office_tools_remove_checkboxes = []
        office_tools_remove_options = [("LibreOffice", "./remove-libreoffice.sh"), ("OnlyOffice", "./remove-onlyoffice.sh")]

        for option, script in office_tools_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_office_tools_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.office_tools_remove_checkboxes.append((check_button, script))

        """ Miscellaneous category - Future use
        frame_miscellaneous_remove = Gtk.Frame(label="Miscellaneous")
        vbox_remove.pack_start(frame_miscellaneous_remove, True, True, 0)

        box_miscellaneous_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_miscellaneous_remove.add(box_miscellaneous_remove)

        self.miscellaneous_remove_checkboxes = []
        miscellaneous_remove_options = [("Any other scripts", "./remove-other.sh")]

        for option, script in miscellaneous_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_miscellaneous_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.miscellaneous_remove_checkboxes.append((check_button, script))
        Miscellaneous category end"""

        # Administration tab
        administration_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(administration_tab, Gtk.Label(label="Administration"))

        # Add a scrolled window to the administration tab
        scrolled_window_administration = Gtk.ScrolledWindow()
        scrolled_window_administration.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        administration_tab.pack_start(scrolled_window_administration, True, True, 0)

        vbox_administration = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        scrolled_window_administration.add(vbox_administration)

        # Power Management category
        frame_power_management = Gtk.Frame(label="Power Management")
        frame_power_management.set_margin_start(10)  # Add margin on the left
        frame_power_management.set_margin_end(10)    # Add margin on the right
        vbox_administration.pack_start(frame_power_management, True, True, 0)

        box_power_management = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_power_management.add(box_power_management)

        self.power_management_checkboxes = []
        power_management_options = [("Enable Bluetooth", "./enable-bluetooth.sh"), ("Disable Bluetooth", "./disable-bluetooth.sh"),
                                    ("TLP (Laptops)", "./install-tlp.sh"), ("Powertop", "./install-powertop.sh")]

        for option, script in power_management_options:
            check_button = Gtk.CheckButton(label=option)
            box_power_management.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.power_management_checkboxes.append((check_button, script))

        # System category
        frame_system = Gtk.Frame(label="System")
        frame_system.set_margin_start(10)  # Add margin on the left
        frame_system.set_margin_end(10)    # Add margin on the right
        vbox_administration.pack_start(frame_system, True, True, 0)

        box_system = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_system.add(box_system)

        self.system_checkboxes = []
        system_options = [("Update System", "./update-system.sh"), ("nala (rank mirrors)", "./install-nala.sh"), 
                        ("Stacer", "./install-stacer.sh"), ("SWAP Fix", "./install-swapfix.sh")]

        for option, script in system_options:
            check_button = Gtk.CheckButton(label=option)
            box_system.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.system_checkboxes.append((check_button, script))

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
        for check_button, script in (self.dev_tools_checkboxes + self.communication_checkboxes + self.browsers_checkboxes +
                                     self.multimedia_checkboxes + self.gaming_checkboxes + self.system_utils_checkboxes +
                                     self.office_tools_checkboxes +
                                     self.dev_tools_remove_checkboxes + self.communication_remove_checkboxes +
                                     self.browsers_remove_checkboxes + self.multimedia_remove_checkboxes +
                                     self.gaming_remove_checkboxes + self.system_utils_remove_checkboxes +
                                     self.office_tools_remove_checkboxes + self.power_management_checkboxes +
                                     self.system_checkboxes):
            if check_button.get_active():
                print("Running script: {}".format(script))
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
