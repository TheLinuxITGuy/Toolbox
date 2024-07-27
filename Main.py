#!/usr/bin/python3
# -*- coding: utf-8 -*-

import gi
import subprocess

gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

class ourwindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy - Linux Mint Scripts")
        Gtk.Window.set_default_size(self, 400, 325)
        Gtk.Window.set_position(self, Gtk.WindowPosition.CENTER)
        # Prevent the window from being resized
        Gtk.Window.set_resizable(self, False)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.add(vbox)

        # Install category
        frame_install = Gtk.Frame(label="Install")
        vbox.pack_start(frame_install, True, True, 0)

        box_install = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_install.add(box_install)

        self.install_checkboxes = []
        install_options = [("Brave Browser", "./install-bravebrowser.sh"), ("Google Chrome", "./install-chromebrowser.sh"), 
                           ("Microsoft Visual Studio Code", "./install-code.sh"), ("Discord", "./install-discord.sh"), 
                           ("Microsoft Edge", "./install-edge.sh"), ("GIMP", "./install-gimp.sh"), 
                           ("Lutris", "./install-lutris.sh"), ("Steam & ProtonUp-Qt", "./install-steam&protonupqt.sh")]

        for option, script in install_options:
            check_button = Gtk.CheckButton(label=option)
            box_install.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.install_checkboxes.append((check_button, script))

        # Add a separator with a height of 10 pixels
        spacer = Gtk.Box()
        spacer.set_size_request(0, 10)  # Set the height of the spacer to 10 pixels
        vbox.pack_start(spacer, False, False, 0)

        # Remove category
        frame_remove = Gtk.Frame(label="Remove")
        vbox.pack_start(frame_remove, True, True, 0)

        box_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_remove.add(box_remove)

        self.remove_checkboxes = []
        remove_options = [("Celluloid", "./remove-celluloid.sh"), ("Firefox", "./remove-firefox.sh"), ("Hypnotix", "./remove-hypnotix.sh"), 
                          ("Libre Office", "./remove-libreoffice.sh"), ("Rhythmbox", "./remove-rhythmbox.sh"), ("Thunderbird Mail", "./remove-thunderbird.sh")]

        for option, script in remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.remove_checkboxes.append((check_button, script))

        # Run and Quit buttons
        hbox = Gtk.Box(spacing=6)
        vbox.pack_start(hbox, True, True, 0)

        run_button = Gtk.Button(label="Run")
        run_button.connect("clicked", self.on_run_button_clicked)
        hbox.pack_start(run_button, True, True, 0)

        quit_button = Gtk.Button(label="Quit")
        quit_button.connect("clicked", Gtk.main_quit)
        hbox.pack_start(quit_button, True, True, 0)

    def on_check_button_toggled(self, button):
        if button.get_active():
            print("{} checked".format(button.get_label()))
        else:
            print("{} unchecked".format(button.get_label()))

    def on_run_button_clicked(self, button):
        print("Run button clicked")
        for check_button, script in self.install_checkboxes + self.remove_checkboxes:
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

window = ourwindow()
window.connect("delete-event", Gtk.main_quit)
window.show_all()
Gtk.main()
