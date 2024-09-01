#!/usr/bin/python3
# -*- coding: utf-8 -*-

import gi
import subprocess
import os

gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

class OurWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="The Linux IT Guy's Toolbox")
        Gtk.Window.set_default_size(self, 1400, 880)  # Doubled the size
        Gtk.Window.set_position(self, Gtk.WindowPosition.CENTER)
        # Prevent the window from being resized
        Gtk.Window.set_resizable(self, True)

        vbox_main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.add(vbox_main)

        # Create a Notebook for tabs
        notebook = Gtk.Notebook()
        vbox_main.pack_start(notebook, True, True, 0)

# -------------------------- INSTALL TAB --------------------------

        # Install tab
        install_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(install_tab, Gtk.Label(label="Install"))

        # Add a scrolled window to the install tab
        scrolled_window_install = Gtk.ScrolledWindow()
        scrolled_window_install.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        install_tab.pack_start(scrolled_window_install, True, True, 0)

        vbox_install = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        scrolled_window_install.add(vbox_install)

        # Linux Kernels category
        frame_dev_tools = Gtk.Frame(label="Linux Kernels")
        frame_dev_tools.set_margin_start(10)  # Add margin on the left
        frame_dev_tools.set_margin_end(10)    # Add margin on the right
        vbox_install.pack_start(frame_dev_tools, True, True, 0)

        box_dev_tools = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools.add(box_dev_tools)

        self.dev_tools_checkboxes = []
        dev_tools_options = [
            ("Install Liquorix Kernel - Installs the Liquorix Kernel, a high-performance kernel optimized for desktop and multimedia use, providing improved responsiveness and better performance for gaming and real-time applications.", "./install-liquorix-kernel.sh"),
            ("Install XanMod Kernel - Installs the XanMod Kernel, a performance-oriented kernel designed for desktop and gaming use, offering enhanced system responsiveness, reduced latency, and improved hardware support.", "./install-xanmod-kernel.sh"),
]


        for option, script in dev_tools_options:
            check_button = Gtk.CheckButton(label=option)
            box_dev_tools.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.dev_tools_checkboxes.append((check_button, script))

        # Development Tools category
        frame_dev_tools = Gtk.Frame(label="Development Tools")
        frame_dev_tools.set_margin_start(10)  # Add margin on the left
        frame_dev_tools.set_margin_end(10)    # Add margin on the right
        vbox_install.pack_start(frame_dev_tools, True, True, 0)

        box_dev_tools = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools.add(box_dev_tools)

        self.dev_tools_checkboxes = []
        dev_tools_options = [
            ("Bottles - Installs Bottles, a tool for managing and running Windows applications on Linux through Wine, providing a user-friendly interface for configuring and managing compatibility settings.", "./install-bottles.sh"),
            ("Boxes - Installs GNOME Boxes, a virtualization tool that simplifies the creation and management of virtual machines, enabling you to run multiple operating systems on a single host.", "./install-boxes.sh"),
            ("Microsoft Visual Studio Code - Installs Visual Studio Code, a lightweight yet powerful code editor with features such as debugging, Git integration, and a rich extension marketplace for enhanced development workflows.", "./install-code.sh"),
            ("PyCharm - Installs PyCharm, a comprehensive integrated development environment (IDE) for Python, offering advanced coding assistance, debugging, and project management tools tailored for Python developers.", "./install-pycharm.sh")
]


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
        communication_options = [
            ("Discord - Installs Discord, a popular communication platform for text, voice, and video chat, widely used by gaming communities and various online groups for collaboration and socializing.", "./install-discord.sh"),
            ("Thunderbird Mail - Installs Thunderbird Mail, an open-source email client that offers powerful features for managing multiple email accounts, organizing emails, and handling calendar events.", "./install-thunderbird.sh"),
            ("Slack - Installs Slack, a messaging and collaboration tool designed for teams, providing real-time chat, file sharing, and integration with various productivity apps and services.", "./install-slack.sh")
]


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
        browsers_options = [
            ("Brave Browser - Installs Brave Browser, a privacy-focused web browser with built-in ad-blocking, tracker prevention, and rewards for viewing privacy-respecting ads.", "./install-bravebrowser.sh"),
            ("Microsoft Edge - Installs Microsoft Edge, a Chromium-based web browser known for its integration with Microsoft services, enhanced performance, and security features.", "./install-edge.sh"),
            ("Google Chrome - Installs Google Chrome, a widely-used web browser with extensive features including synchronization with Google services, a large extension library, and advanced security.", "./install-chromebrowser.sh"),
            ("Firefox - Installs Firefox, an open-source web browser renowned for its privacy features, customizability, and support for a wide range of web standards.", "./install-firefox.sh"),
            ("Opera - Installs Opera, a web browser with built-in features such as a free VPN, ad blocker, and integrated messaging apps, offering a customizable user experience.", "./install-opera.sh"),
            ("Vivaldi - Installs Vivaldi, a highly customizable web browser that provides unique features like built-in tools for note-taking and advanced tab management, tailored to power users.", "./install-vivaldi.sh")
]


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
        multimedia_options = [
            ("GIMP - Installs GIMP, the GNU Image Manipulation Program, a powerful open-source tool for photo editing, image creation, and graphic design with extensive features and plugins.", "./install-gimp.sh"),
            ("OBS-Studio - Installs OBS-Studio, an open-source software for video recording and live streaming, offering high-performance features for capturing and broadcasting video content.", "./install-obs.sh"),
            ("Audacity - Installs Audacity, a free and open-source audio editing software that provides a wide range of tools for recording, editing, and processing audio files.", "./install-audacity.sh")
]


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
        gaming_options = [
            ("Lutris - Installs Lutris, a gaming platform that helps manage and run games from various sources, including native Linux games and Windows games through Wine and other compatibility layers.", "./install-lutris.sh"),
            ("Steam & ProtonUp-Qt - Installs Steam, a major digital distribution platform for games, along with ProtonUp-Qt, a tool for managing Proton versions to run Windows games on Linux.", "./install-steam&protonupqt.sh")
]


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
        system_utils_options = [
            ("htop - Installs htop, an interactive process viewer and system monitor that provides a dynamic, real-time display of system processes and resource usage.", "./install-htop.sh"),
            ("Neofetch - Installs Neofetch, a command-line utility that shows system information in a visually appealing format, useful for diagnostics and system reporting.", "./install-neofetch.sh"),
            ("GParted - Installs GParted, a graphical partition editor that allows you to manage disk partitions and filesystems with features for resizing, creating, and deleting partitions.", "./install-gparted.sh"),
            ("QEMU - Installs QEMU, a versatile open-source hardware virtualization tool that enables you to create and run virtual machines, and emulate various hardware platforms.", "./install-QEMU.sh")
]


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
        office_tools_options = [
            ("LibreOffice - Installs LibreOffice, a comprehensive open-source office suite that includes tools for word processing, spreadsheets, presentations, and more.", "./install-libreoffice.sh"),
            ("OnlyOffice - Installs OnlyOffice, an office suite designed for document editing, spreadsheets, and presentations, with features for collaboration and integration with cloud services.", "./install-onlyoffice.sh")
]


        for option, script in office_tools_options:
            check_button = Gtk.CheckButton(label=option)
            box_office_tools.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.office_tools_checkboxes.append((check_button, script))

# -------------------------- REMOVE TAB --------------------------

        # Remove tab
        remove_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(remove_tab, Gtk.Label(label="Remove"))

        # Add a scrolled window to the remove tab
        scrolled_window_remove = Gtk.ScrolledWindow()
        scrolled_window_remove.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        remove_tab.pack_start(scrolled_window_remove, True, True, 0)

        vbox_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        scrolled_window_remove.add(vbox_remove)

        # Development Tools category
        frame_dev_tools_remove = Gtk.Frame(label="Development Tools")
        frame_dev_tools_remove.set_margin_start(10)
        frame_dev_tools_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_dev_tools_remove, True, True, 0)

        box_dev_tools_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools_remove.add(box_dev_tools_remove)

        self.dev_tools_remove_checkboxes = []
        dev_tools_remove_options = [
            ("Remove Liquorix Kernel - Removes the Liquorix Kernel, a high-performance kernel optimized for desktop and multimedia use, providing improved responsiveness and better performance for gaming and real-time applications.", "./remove-liquorix-kernel.sh"),
            ("Remove XanMod Kernel - Removes the XanMod Kernel, a performance-oriented kernel designed for desktop and gaming use, offering enhanced system responsiveness, reduced latency, and improved hardware support.", "./remove-xanmod-kernel.sh")
]

        for option, script in dev_tools_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_dev_tools_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.dev_tools_remove_checkboxes.append((check_button, script))

        # Development Tools category
        frame_dev_tools_remove = Gtk.Frame(label="Development Tools")
        frame_dev_tools_remove.set_margin_start(10)
        frame_dev_tools_remove.set_margin_end(10)
        vbox_remove.pack_start(frame_dev_tools_remove, True, True, 0)

        box_dev_tools_remove = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_dev_tools_remove.add(box_dev_tools_remove)

        self.dev_tools_remove_checkboxes = []
        dev_tools_remove_options = [
            ("Bottles - Uninstalls Bottles, a tool for managing and running Windows applications on Linux, including all configurations and virtual environments associated with it.", "./remove-bottles.sh"),
            ("Boxes - Removes GNOME Boxes, a virtualization tool for managing virtual machines, including all virtual machines and associated data.", "./remove-boxes.sh"),
            ("Microsoft Visual Studio Code - Uninstalls Visual Studio Code, a popular code editor, including all extensions, settings, and user configurations.", "./remove-code.sh"),
            ("PyCharm - Uninstalls PyCharm, an integrated development environment (IDE) for Python, removing all project files, configurations, and associated settings.", "./remove-pycharm.sh")
]


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
        communication_remove_options = [
            ("Discord - Uninstalls Discord, a communication platform for voice, video, and text chat, including all user data, settings, and associated files.", "./remove-discord.sh"),
            ("Thunderbird Mail - Removes Thunderbird Mail, an open-source email client, including all emails, accounts, and configurations stored on your system.", "./remove-thunderbird.sh"),
            ("Slack - Uninstalls Slack, a messaging and collaboration app for teams, removing all user data, workspace settings, and associated files.", "./remove-slack.sh")
]


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
        browsers_remove_options = [
            ("Brave Browser - Uninstalls Brave Browser, a privacy-focused web browser, including all user data, bookmarks, and settings associated with it.", "./remove-bravebrowser.sh"),
            ("Microsoft Edge - Removes Microsoft Edge, a Chromium-based browser from Microsoft, along with all associated data, settings, and profiles.", "./remove-edge.sh"),
            ("Google Chrome - Uninstalls Google Chrome, a widely-used web browser, including all user data, extensions, and settings.", "./remove-chromebrowser.sh"),
            ("Firefox - Removes Firefox, an open-source web browser known for its privacy features, along with all user profiles, bookmarks, and extensions.", "./remove-firefox.sh"),
            ("Opera - Uninstalls Opera, a web browser with built-in features such as a VPN and ad blocker, including all user data and custom settings.", "./remove-opera.sh"),
            ("Vivaldi - Removes Vivaldi, a customizable web browser with a focus on user control, including all browsing data, settings, and extensions.", "./remove-vivaldi.sh")
]


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
        multimedia_remove_options = [
            ("GIMP - Uninstalls GIMP, the GNU Image Manipulation Program, including all related plugins and configuration files used for image editing and graphic design.", "./remove-gimp.sh"),
            ("OBS-Studio - Removes OBS-Studio, a software for video recording and live streaming, along with its settings, configurations, and any installed plugins.", "./remove-obs.sh"),
            ("Audacity - Uninstalls Audacity, an open-source audio editing tool, including all related libraries, plugins, and user data used for recording and editing audio files.", "./remove-audacity.sh")
]

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
        gaming_remove_options = [
            ("Lutris - Uninstalls Lutris, a gaming platform that helps manage and run games from various sources, including configurations and related data.", "./remove-lutris.sh"),
            ("Steam & ProtonUp-Qt - Removes Steam and ProtonUp-Qt, including all associated game files, settings, and the Proton compatibility tool used for running Windows games on Linux.", "./remove-steam&protonupqt.sh")
]


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
        system_utils_remove_options = [
            ("htop - Uninstalls htop, an interactive process viewer and system monitor that provides a dynamic, real-time display of system processes and resource usage.", "./remove-htop.sh"),
            ("Neofetch - Removes Neofetch, a command-line utility that displays system information in a visually appealing manner for diagnostics and reporting purposes.", "./remove-neofetch.sh"),
            ("GParted - Uninstalls GParted, a graphical partition editor for managing disk partitions, including all associated files and configuration settings.", "./remove-gparted.sh"),
            ("QEMU - Removes QEMU, a versatile open-source hardware virtualization tool, along with its related components and virtual machine data.", "./remove-QEMU.sh")
]


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
        office_tools_remove_options = [
            ("LibreOffice - Removes the LibreOffice suite, including all associated applications like Writer, Calc, and Impress, as well as configuration files and user data.", "./remove-libreoffice.sh"),
            ("OnlyOffice - Uninstalls OnlyOffice, including all its components for document editing, spreadsheets, and presentations, along with related files and settings.", "./remove-onlyoffice.sh")
]


        for option, script in office_tools_remove_options:
            check_button = Gtk.CheckButton(label=option)
            box_office_tools_remove.pack_start(check_button, True, True, 0)
            check_button.connect("toggled", self.on_check_button_toggled)
            self.office_tools_remove_checkboxes.append((check_button, script))

# -------------------------- ADMINISTRATION TAB --------------------------

        # Administration tab
        administration_tab = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        notebook.append_page(administration_tab, Gtk.Label(label="Administration"))

        # Add a scrolled window to the administration tab
        scrolled_window_administration = Gtk.ScrolledWindow()
        scrolled_window_administration.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        administration_tab.pack_start(scrolled_window_administration, True, True, 0)

        vbox_administration = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        scrolled_window_administration.add(vbox_administration)

        # Power Management category
        frame_power_management = Gtk.Frame(label="Power Management")
        frame_power_management.set_margin_start(10)  # Add margin on the left
        frame_power_management.set_margin_end(10)    # Add margin on the right
        vbox_administration.pack_start(frame_power_management, True, True, 0)

        box_power_management = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        frame_power_management.add(box_power_management)

        self.power_management_checkboxes = []
        power_management_options = [
            ("Enable Bluetooth - Activates Bluetooth functionality, allowing your system to connect to and communicate with Bluetooth devices.", "./enable-bluetooth.sh"),
            ("Disable Bluetooth - Deactivates Bluetooth functionality to save power and prevent unwanted connections or interference.", "./disable-bluetooth.sh"),
            ("TLP (Laptops) - Installs TLP, a power management tool specifically designed for laptops to enhance battery life by optimizing various power-related settings.", "./install-tlp.sh"),
            ("Powertop - Installs Powertop, a utility for monitoring and optimizing power consumption, helping you identify and reduce power usage by various system components and processes.", "./install-powertop.sh")
]


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
        system_options = [
            ("Update System - Updates all installed packages and dependencies to their latest versions, ensuring your system is current and secure.", "./update-system.sh"),
            ("Nala (rank mirrors) - Installs Nala and ranks mirror servers to enhance package download speeds and update efficiency by choosing the best available mirrors.", "./install-nala.sh"),
            ("Stacer - Installs Stacer, a comprehensive system optimizer and monitoring tool that helps manage system performance, startup applications, and more through an intuitive graphical interface.", "./install-stacer.sh"),
            ("SWAP Fix - Installs fixes for common issues with swap memory, optimizing your system's performance by adjusting swap settings and resolving related problems.", "./install-swapfix.sh"),
            ("System Cleanup - Performs a thorough cleanup of the system by removing caches, orphaned packages, and other unnecessary files to free up disk space and improve system performance.", "./system-cleanup.sh"),
            ("Enable ZRAM - Activates ZRAM for improved memory management and system performance by configuring and enabling ZRAM swap space.", "./enable-zram.sh"),
            ("Disable ZRAM - Deactivates ZRAM and removes related configurations and packages to restore default memory management settings.", "./disable-zram.sh")

]

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
            text="All tasks are completed. Some changes might require a restart to take effect",
        )
        dialog.format_secondary_text(
            "Please click the 'Quit' button to exit the application."
        )

        # Add a button next to OK to restart system
        dialog.add_button("Restart", Gtk.ResponseType.REJECT)
        dialog.connect("response", self.on_dialog_response)
        dialog.show_all()

        dialog.run()
        dialog.destroy()

    def on_dialog_response(self, dialog, response_id):
        if response_id == Gtk.ResponseType.REJECT:
            os.system("sudo reboot")

window = OurWindow()
window.connect("delete-event", Gtk.main_quit)
window.show_all()
Gtk.main()