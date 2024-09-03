#!/usr/bin/python3
# -*- coding: utf-8 -*-

import gi
import subprocess
import os

gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk

class SettingsWindow(Gtk.Window):
    def __init__(self, main_window):
        Gtk.Window.__init__(self, title="Settings")
        self.set_default_size(400, 300)
        self.main_window = main_window
        
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        self.add(vbox)

        # Font Size Adjustment
        font_size_label = Gtk.Label(label="Font Size:")
        self.font_size_adjustment = Gtk.Adjustment(12, 8, 48, 1, 10, 0)
        font_size_spin_button = Gtk.SpinButton(adjustment=self.font_size_adjustment)
        font_size_spin_button.connect("value-changed", self.on_font_size_changed)

        vbox.pack_start(font_size_label, False, False, 0)
        vbox.pack_start(font_size_spin_button, False, False, 0)

        # Font Color
        font_color_label = Gtk.Label(label="Font Color:")
        self.font_color_button = Gtk.ColorButton()
        self.font_color_button.connect("color-set", self.on_font_color_changed)

        vbox.pack_start(font_color_label, False, False, 0)
        vbox.pack_start(self.font_color_button, False, False, 0)

        # Background Color
        bg_color_label = Gtk.Label(label="Background Color:")
        self.bg_color_button = Gtk.ColorButton()
        self.bg_color_button.connect("color-set", self.on_bg_color_changed)

        vbox.pack_start(bg_color_label, False, False, 0)
        vbox.pack_start(self.bg_color_button, False, False, 0)

    def on_font_size_changed(self, widget):
        font_size = self.font_size_adjustment.get_value()
        style_provider = Gtk.CssProvider()
        css = f"* {{ font-size: {font_size}px; }}"
        style_provider.load_from_data(css.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(), 
            style_provider, 
            Gtk.STYLE_PROVIDER_PRIORITY_USER
        )

    def on_font_color_changed(self, widget):
        color = self.font_color_button.get_rgba()
        css_color = f"rgba({int(color.red * 255)}, {int(color.green * 255)}, {int(color.blue * 255)}, {color.alpha})"
        style_provider = Gtk.CssProvider()
        css = f"* {{ color: {css_color}; }}"
        style_provider.load_from_data(css.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_USER
        )

    def on_bg_color_changed(self, widget):
        color = self.bg_color_button.get_rgba()
        css_color = f"rgba({int(color.red * 255)}, {int(color.green * 255)}, {int(color.blue * 255)}, {color.alpha})"
        style_provider = Gtk.CssProvider()
        css = f"* {{ background-color: {css_color}; }}"
        style_provider.load_from_data(css.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_USER
        )

class OurWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy's Toolbox")
        self.set_default_size(800, 650)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_resizable(True)

        vbox_main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        vbox_main.set_margin_top(15)
        vbox_main.set_margin_bottom(15)
        vbox_main.set_margin_start(15)
        vbox_main.set_margin_end(15)
        self.add(vbox_main)

        # Create a Notebook for tabs
        notebook = Gtk.Notebook()
        vbox_main.pack_start(notebook, True, True, 0)

        # Install tab
        self.dev_tools_checkboxes = []
        self.communication_checkboxes = []
        self.browsers_checkboxes = []
        self.multimedia_checkboxes = []
        self.gaming_checkboxes = []
        self.system_utils_checkboxes = []
        self.office_tools_checkboxes = []
        
        install_tab = self.create_tab("Install", self.get_install_options())
        notebook.append_page(install_tab, Gtk.Label(label="Install"))

        # Remove tab
        self.dev_tools_remove_checkboxes = []
        self.communication_remove_checkboxes = []
        self.browsers_remove_checkboxes = []
        self.multimedia_remove_checkboxes = []
        self.gaming_remove_checkboxes = []
        self.system_utils_remove_checkboxes = []
        self.office_tools_remove_checkboxes = []
        
        remove_tab = self.create_tab("Remove", self.get_remove_options())
        notebook.append_page(remove_tab, Gtk.Label(label="Remove"))

        # Administration tab
        self.power_management_checkboxes = []
        self.system_checkboxes = []
        
        administration_tab = self.create_tab("Administration", self.get_admin_options())
        notebook.append_page(administration_tab, Gtk.Label(label="Administration"))

        # Run, Quit, and Settings buttons
        hbox_buttons = Gtk.Box(spacing=10)
        vbox_main.pack_start(hbox_buttons, False, False, 0)

        run_button = Gtk.Button(label="Run")
        run_button.set_image(Gtk.Image.new_from_icon_name("media-playback-start", Gtk.IconSize.BUTTON))
        run_button.connect("clicked", self.on_run_button_clicked)
        hbox_buttons.pack_start(run_button, True, True, 0)

        quit_button = Gtk.Button(label="Quit")
        quit_button.set_image(Gtk.Image.new_from_icon_name("application-exit", Gtk.IconSize.BUTTON))
        quit_button.connect("clicked", Gtk.main_quit)
        hbox_buttons.pack_start(quit_button, True, True, 0)

        settings_button = Gtk.Button(label="Settings")
        settings_button.set_image(Gtk.Image.new_from_icon_name("preferences-system", Gtk.IconSize.BUTTON))
        settings_button.connect("clicked", self.on_settings_button_clicked)
        hbox_buttons.pack_start(settings_button, True, True, 0)

    def on_settings_button_clicked(self, widget):
        settings_window = SettingsWindow(self)
        settings_window.show_all()

    def create_tab(self, title, category_options):
        tab_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        
        # Add a scrolled window to the tab
        scrolled_window = Gtk.ScrolledWindow()
        scrolled_window.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        tab_box.pack_start(scrolled_window, True, True, 0)

        vbox_tab_content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        scrolled_window.add(vbox_tab_content)

        # Add categories to the tab
        for category in category_options:
            frame = Gtk.Frame(label=category['label'])
            frame.set_margin_start(10)
            frame.set_margin_end(10)
            frame.set_margin_top(10)
            frame.set_margin_bottom(10)
            vbox_tab_content.pack_start(frame, False, False, 0)

            category_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
            frame.add(category_box)

            for option, script in category['options']:
                check_button = Gtk.CheckButton(label=option)
                check_button.connect("toggled", self.on_check_button_toggled)
                category_box.pack_start(check_button, False, False, 0)
                category['checkboxes'].append((check_button, script))
        
        return tab_box
    
    def get_install_options(self):
        return [
            {'label': "Development Tools", 'checkboxes': self.dev_tools_checkboxes, 'options': [
                ("Bottles", "./install-bottles.sh"), 
                ("Boxes", "./install-boxes.sh"),
                ("Microsoft Visual Studio Code", "./install-code.sh"), 
                ("PyCharm", "./install-pycharm.sh"),
            ]},
            {'label': "Communication", 'checkboxes': self.communication_checkboxes, 'options': [
                ("Discord", "./install-discord.sh"), 
                ("Thunderbird Mail", "./install-thunderbird.sh"), 
                ("Slack", "./install-slack.sh"),
            ]},
            {'label': "Browsers", 'checkboxes': self.browsers_checkboxes, 'options': [
                ("Brave Browser", "./install-bravebrowser.sh"), 
                ("Microsoft Edge", "./install-edge.sh"),
                ("Google Chrome", "./install-chromebrowser.sh"), 
                ("Firefox", "./install-firefox.sh"), 
                ("Opera", "./install-opera.sh"), 
                ("Vivaldi", "./install-vivaldi.sh"),
            ]},
            {'label': "Multimedia", 'checkboxes': self.multimedia_checkboxes, 'options': [
                ("GIMP", "./install-gimp.sh"), 
                ("OBS-Studio", "./install-obs.sh"), 
                ("Audacity", "./install-audacity.sh"),
            ]},
            {'label': "Gaming", 'checkboxes': self.gaming_checkboxes, 'options': [
                ("Lutris", "./install-lutris.sh"), 
                ("Steam & ProtonUp-Qt", "./install-steam&protonupqt.sh"),
            ]},
            {'label': "Utilities", 'checkboxes': self.system_utils_checkboxes, 'options': [
                ("htop", "./install-htop.sh"), 
                ("Neofetch", "./install-neofetch.sh"), 
                ("GParted", "./install-gparted.sh"), 
                ("QEMU", "./install-QEMU.sh"),
            ]},
            {'label': "Office Tools", 'checkboxes': self.office_tools_checkboxes, 'options': [
                ("LibreOffice", "./install-libreoffice.sh"), 
                ("OnlyOffice", "./install-onlyoffice.sh"),
            ]},
        ]

    def get_remove_options(self):
        return [
            {'label': "Development Tools", 'checkboxes': self.dev_tools_remove_checkboxes, 'options': [
                ("Bottles", "./remove-bottles.sh"), 
                ("Boxes", "./remove-boxes.sh"),
                ("Microsoft Visual Studio Code", "./remove-code.sh"), 
                ("PyCharm", "./remove-pycharm.sh"),
            ]},
            {'label': "Communication", 'checkboxes': self.communication_remove_checkboxes, 'options': [
                ("Discord", "./remove-discord.sh"), 
                ("Thunderbird Mail", "./remove-thunderbird.sh"), 
                ("Slack", "./remove-slack.sh"),
            ]},
            {'label': "Browsers", 'checkboxes': self.browsers_remove_checkboxes, 'options': [
                ("Brave Browser", "./remove-bravebrowser.sh"), 
                ("Microsoft Edge", "./remove-edge.sh"),
                ("Google Chrome", "./remove-chromebrowser.sh"), 
                ("Firefox", "./remove-firefox.sh"), 
                ("Opera", "./remove-opera.sh"), 
                ("Vivaldi", "./remove-vivaldi.sh"),
            ]},
            {'label': "Multimedia", 'checkboxes': self.multimedia_remove_checkboxes, 'options': [
                ("GIMP", "./remove-gimp.sh"), 
                ("OBS-Studio", "./remove-obs.sh"), 
                ("Audacity", "./remove-audacity.sh"),
            ]},
            {'label': "Gaming", 'checkboxes': self.gaming_remove_checkboxes, 'options': [
                ("Lutris", "./remove-lutris.sh"), 
                ("Steam & ProtonUp-Qt", "./remove-steam&protonupqt.sh")
            ]},
            {'label': "Utilities", 'checkboxes': self.system_utils_remove_checkboxes, 'options': [
                ("htop", "./remove-htop.sh"), 
                ("Neofetch", "./remove-neofetch.sh"), 
                ("GParted", "./remove-gparted.sh"), 
                ("QEMU", "./remove-QEMU.sh"),
            ]},
            {'label': "Office Tools", 'checkboxes': self.office_tools_remove_checkboxes, 'options': [
                ("LibreOffice", "./remove-libreoffice.sh"), 
                ("OnlyOffice", "./remove-onlyoffice.sh"),
            ]},
        ]

    def get_admin_options(self):
        return [
            {'label': "Power Management", 'checkboxes': self.power_management_checkboxes, 'options': [
                ("Install TLP", "./install-tlp.sh"),
                ("Remove TLP", "./remove-tlp.sh"),
                ("Install Powertop", "./install-powertop.sh"),
                ("Remove Powertop", "remove-powertop.sh"),
                ("Install Stacer", "./install-stacer.sh"),
                ("Remove Stacer", "./remove-stacer.sh")
            ]},
            {'label': "System", 'checkboxes': self.system_checkboxes, 'options': [
                ("Update System", "./update-system.sh"),
                ("Enable ZRAM", "./enable-zram.sh"),
                ("Disable ZRAM", "./disable-zram.sh"),
                ("Enable Bluetooth", "./enable-bluetooth.sh"),
                ("Disable Bluetooth", "./disable-bluetooth.sh"),
                ("Swap Fix", "./install-swapfix.sh"),
                ("Nala (Rank Mirrors)", "./install-nala.sh"),
                ("System Cleanup", "./system-cleanup.sh"),
            ]},
        ]

    def on_check_button_toggled(self, check_button):
        pass  # No operation on toggle for now

    def on_run_button_clicked(self, widget):
        for check_button, script in (
            self.dev_tools_checkboxes + self.communication_checkboxes + self.browsers_checkboxes +
            self.multimedia_checkboxes + self.gaming_checkboxes + self.system_utils_checkboxes +
            self.office_tools_checkboxes + self.dev_tools_remove_checkboxes + self.communication_remove_checkboxes +
            self.browsers_remove_checkboxes + self.multimedia_remove_checkboxes + self.gaming_remove_checkboxes +
            self.system_utils_remove_checkboxes + self.office_tools_remove_checkboxes + self.power_management_checkboxes +
            self.system_checkboxes
        ):
            if check_button.get_active():
                print(f"Executing: {script}")
                subprocess.call(["bash", script])

        # Call method to show the completion dialog
        self.show_completion_dialog()

    def show_completion_dialog(self):
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=0,
            message_type=Gtk.MessageType.INFO,
            buttons=Gtk.ButtonsType.NONE,
            text="Execution Complete",
        )
        dialog.format_secondary_text(
            "All selected tasks have been executed. Do you want to restart the computer now?"
        )
        
        # Add Restart and OK buttons
        dialog.add_buttons(Gtk.STOCK_OK, Gtk.ResponseType.OK, "Restart", Gtk.ResponseType.CANCEL)
        
        response = dialog.run()

        if response == Gtk.ResponseType.OK:
            dialog.destroy()
        elif response == Gtk.ResponseType.CANCEL:
            dialog.destroy()
            self.restart_computer()

    def restart_computer(self):
        print("Restarting the computer...")
        subprocess.call(["sudo", "reboot"])

win = OurWindow()
win.connect("destroy", Gtk.main_quit)
win.show_all()
Gtk.main()
