#!/usr/bin/env python3

import gi
import os
import re
import subprocess
import shutil

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Gio, GLib, Adw

# Common paths and settings
IGNORE_DIRS = {
    "Microsoft",
    "Temp",
    "Packages",
    "ConnectedDevicesPlatform",
    "Comms",
    "Apps",
}
SAVE_PATHS = [
    "AppData/Local",
    "AppData/LocalLow",
    "AppData/Roaming",
    "Saved Games",
]


class CompatDataPage(Gtk.Box):
    def __init__(self, window):
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        self.window = window
        self.set_margin_top(12)
        self.set_margin_bottom(12)
        self.set_margin_start(12)
        self.set_margin_end(12)
        self.set_spacing(12)

        # Header
        header = Gtk.Label(label="Proton Compatdata Folders")
        header.add_css_class("title-1")
        self.append(header)

        # Description
        description = Gtk.Label(label="Manage your Proton prefixes and game save files")
        description.add_css_class("subtitle-1")
        self.append(description)

        # Refresh button
        refresh_button = Gtk.Button(label="Refresh")
        refresh_button.connect("clicked", self.refresh_data)
        self.append(refresh_button)

        # Create scrolled window for the list
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        self.append(scroll)

        # Main list
        self.listbox = Gtk.ListBox()
        self.listbox.set_selection_mode(Gtk.SelectionMode.NONE)
        self.listbox.add_css_class("boxed-list")
        scroll.set_child(self.listbox)

        # Load initial data
        self.refresh_data()

    def open_file_manager(self, button, path):
        if os.path.exists(path):
            subprocess.Popen(["xdg-open", path])
        else:
            self.show_error_dialog(f"Path does not exist: {path}")

    def delete_prefix(self, button, prefix_path, game_id, row):
        # Create confirmation dialog
        dialog = Adw.MessageDialog.new(
            self.window,
            f"Delete Prefix for Game ID {game_id}?",
            "This will permanently delete the prefix folder and all save files. This action cannot be undone.",
        )
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("delete", "Delete")
        dialog.set_response_appearance("delete", Adw.ResponseAppearance.DESTRUCTIVE)

        dialog.connect("response", self.on_delete_confirmed, prefix_path, row)
        dialog.present()

    def on_delete_confirmed(self, dialog, response, prefix_path, row):
        if response == "delete":
            try:
                shutil.rmtree(prefix_path)
                self.listbox.remove(row)
            except Exception as e:
                self.show_error_dialog(f"Error deleting prefix: {e}")

    def show_error_dialog(self, message):
        dialog = Adw.MessageDialog.new(self.window, "Error", message)
        dialog.add_response("ok", "OK")
        dialog.present()

    def refresh_data(self, button=None):
        # Clear existing items
        while True:
            child = self.listbox.get_first_child()
            if child is None:
                break
            self.listbox.remove(child)

        # Scan for compatdata folders
        base_path = os.path.expanduser("~/.steam/steam/steamapps/compatdata")

        if not os.path.isdir(base_path):
            error_label = Gtk.Label(label="No compatdata directory found.")
            error_label.set_margin_top(12)
            error_label.set_margin_bottom(12)
            error_label.set_margin_start(12)
            error_label.set_margin_end(12)
            self.listbox.append(error_label)
            return

        # Sort the game IDs numerically
        game_ids = sorted(
            os.listdir(base_path), key=lambda x: int(x) if x.isdigit() else float("inf")
        )

        for game_id in game_ids:
            compat_path = os.path.join(base_path, game_id)
            pfx_path = os.path.join(compat_path, "pfx")
            drive_c_path = os.path.join(pfx_path, "drive_c")
            user_path = os.path.join(drive_c_path, "users", "steamuser")

            if not os.path.isdir(pfx_path):
                continue

            # Create an expander for each game prefix
            expander = Gtk.Expander(label=f"Game ID: {game_id}")
            expander.set_margin_top(8)
            expander.set_margin_bottom(8)
            expander.set_margin_start(8)
            expander.set_margin_end(8)

            # Create content for the expander
            content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
            content_box.set_margin_start(16)
            content_box.set_margin_top(8)

            # Actions box for the game prefix
            actions_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            actions_box.set_margin_top(8)
            actions_box.set_margin_bottom(8)
            actions_box.set_margin_start(8)
            actions_box.set_margin_end(8)

            # Open drive_c button
            open_drive_c = Gtk.Button(label="Open drive_c Folder")
            open_drive_c.connect("clicked", self.open_file_manager, drive_c_path)
            actions_box.append(open_drive_c)

            # Delete button
            delete_button = Gtk.Button(label="Delete Prefix")
            delete_button.add_css_class("destructive-action")
            row = Gtk.ListBoxRow()
            row.set_child(expander)
            delete_button.connect(
                "clicked", self.delete_prefix, compat_path, game_id, row
            )
            actions_box.append(delete_button)

            content_box.append(actions_box)

            # Separator
            separator = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
            content_box.append(separator)

            # Add save locations
            save_locations_label = Gtk.Label(label="Save Locations:")
            save_locations_label.set_halign(Gtk.Align.START)
            save_locations_label.set_margin_top(8)
            content_box.append(save_locations_label)

            # Create list for save locations
            save_list = Gtk.ListBox()
            save_list.add_css_class("boxed-list")
            save_list.set_selection_mode(Gtk.SelectionMode.NONE)
            content_box.append(save_list)

            found_any_saves = False

            for rel_path in SAVE_PATHS:
                full_path = os.path.join(user_path, rel_path)
                if os.path.isdir(full_path):
                    save_box = Gtk.Box(
                        orientation=Gtk.Orientation.HORIZONTAL, spacing=8
                    )
                    save_box.set_margin_top(8)
                    save_box.set_margin_bottom(8)
                    save_box.set_margin_start(8)
                    save_box.set_margin_end(8)

                    path_label = Gtk.Label(label=rel_path)
                    path_label.set_halign(Gtk.Align.START)
                    path_label.set_hexpand(True)
                    save_box.append(path_label)

                    open_button = Gtk.Button(label="Open")
                    open_button.connect("clicked", self.open_file_manager, full_path)
                    save_box.append(open_button)

                    save_list_row = Gtk.ListBoxRow()
                    save_list_row.set_child(save_box)
                    save_list.append(save_list_row)

                    # Scan for game-specific folders
                    if os.path.isdir(full_path):
                        has_folders = False
                        for entry in sorted(os.listdir(full_path)):
                            entry_path = os.path.join(full_path, entry)
                            if os.path.isdir(entry_path) and entry not in IGNORE_DIRS:
                                has_folders = True
                                found_any_saves = True
                                game_save_box = Gtk.Box(
                                    orientation=Gtk.Orientation.HORIZONTAL, spacing=8
                                )
                                game_save_box.set_margin_top(8)
                                game_save_box.set_margin_bottom(8)
                                game_save_box.set_margin_start(24)
                                game_save_box.set_margin_end(8)

                                save_name = Gtk.Label(label=entry)
                                save_name.set_halign(Gtk.Align.START)
                                save_name.set_hexpand(True)
                                game_save_box.append(save_name)

                                open_save_button = Gtk.Button(label="Open")
                                open_save_button.connect(
                                    "clicked", self.open_file_manager, entry_path
                                )
                                game_save_box.append(open_save_button)

                                save_item_row = Gtk.ListBoxRow()
                                save_item_row.set_child(game_save_box)
                                save_list.append(save_item_row)

            if not found_any_saves:
                no_saves_label = Gtk.Label(label="No save folders found for this game")
                no_saves_label.set_margin_top(8)
                no_saves_label.set_margin_bottom(8)
                no_saves_label.set_margin_start(8)
                no_saves_label.set_margin_end(8)
                content_box.append(no_saves_label)

            expander.set_child(content_box)

            # Add the row to the main list
            self.listbox.append(row)


class SyncPage(Gtk.Box):
    def __init__(self):
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        self.set_margin_top(12)
        self.set_margin_bottom(12)
        self.set_margin_start(12)
        self.set_margin_end(12)

        # Header
        header = Gtk.Label(label="Save Game Sync")
        header.add_css_class("title-1")
        self.append(header)

        # Description
        description = Gtk.Label(
            label="This page will be used for sync functionality (coming soon)"
        )
        description.add_css_class("subtitle-1")
        self.append(description)

        # Empty space
        placeholder = Gtk.Label(label="")
        placeholder.set_vexpand(True)
        self.append(placeholder)


class ProtonSavesWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Proton Game Saves Manager")
        self.set_default_size(900, 700)

        # Create header bar
        header_bar = Adw.HeaderBar()

        # Add title widget for the header bar
        title_widget = Adw.WindowTitle()
        title_widget.set_title("Proton Game Saves Manager")
        title_widget.set_subtitle("Manage your Steam Proton save files")
        header_bar.set_title_widget(title_widget)

        # Add menu button
        menu_button = Gtk.MenuButton()
        menu_button.set_icon_name("open-menu-symbolic")

        # Create the menu
        menu = Gio.Menu()
        menu.append("About", "app.about")
        menu.append("Quit", "app.quit")

        menu_button.set_menu_model(menu)
        header_bar.pack_end(menu_button)

        # Create the main box that will hold everything
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

        # Add the header bar to the main box
        main_box.append(header_bar)

        # Create main content box
        self.content_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.content_box.set_vexpand(True)
        main_box.append(self.content_box)

        # Set the main box as the content of the window
        self.set_content(main_box)

        # Create a split view
        self.split_view = Adw.Leaflet()
        self.split_view.set_can_navigate_back(True)
        self.split_view.set_can_unfold(True)
        self.content_box.append(self.split_view)

        # Create sidebar
        self.sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.sidebar.set_size_request(200, -1)
        self.sidebar.add_css_class("sidebar")

        # Create stack for the content
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.SLIDE_LEFT_RIGHT)
        self.stack.set_transition_duration(200)

        # Create sidebar list
        self.sidebar_list = Gtk.ListBox()
        self.sidebar_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.sidebar_list.add_css_class("navigation-sidebar")
        self.sidebar_list.connect("row-selected", self.on_sidebar_item_selected)
        self.sidebar.append(self.sidebar_list)

        # Add pages to the stack
        compat_page = CompatDataPage(self)
        self.stack.add_titled(compat_page, "compat", "Compatdata Folders")

        sync_page = SyncPage()
        self.stack.add_titled(sync_page, "sync", "Save Sync")

        # Add sidebar items
        self.add_sidebar_item("compat", "Compatdata Folders")
        self.add_sidebar_item("sync", "Save Sync")

        # Add sidebar and stack to the split view
        self.split_view.append(self.sidebar)
        self.split_view.append(self.stack)

        # Select first item
        self.sidebar_list.select_row(self.sidebar_list.get_row_at_index(0))

        # Add application actions
        self.create_actions(app)

    def add_sidebar_item(self, name, title):
        row = Gtk.ListBoxRow()
        label = Gtk.Label(label=title)
        label.set_margin_top(12)
        label.set_margin_bottom(12)
        label.set_margin_start(12)
        label.set_margin_end(12)
        label.set_halign(Gtk.Align.START)
        row.set_child(label)
        row.name = name
        self.sidebar_list.append(row)

    def on_sidebar_item_selected(self, list_box, row):
        if row is not None:
            self.stack.set_visible_child_name(row.name)

    def create_actions(self, app):
        # Add actions
        quit_action = Gio.SimpleAction.new("quit", None)
        quit_action.connect("activate", lambda *_: app.quit())
        app.add_action(quit_action)

        about_action = Gio.SimpleAction.new("about", None)
        about_action.connect("activate", self.on_about_action)
        app.add_action(about_action)

    def on_about_action(self, action, param):
        about = Adw.AboutWindow(transient_for=self)
        about.set_application_name("Proton Game Saves Manager")
        about.set_version("1.0")
        about.set_developer_name("Proton Game Saves Manager Team")
        about.set_license_type(Gtk.License.GPL_3_0)
        about.set_comments("Manage your Steam Proton game save files")
        about.set_website("https://github.com/yourusername/proton-gamesaves")
        about.set_issue_url("https://github.com/yourusername/proton-gamesaves/issues")
        about.present()


class ProtonSavesApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="com.github.proton.gamesaves",
            flags=Gio.ApplicationFlags.FLAGS_NONE,
        )

    def do_activate(self):
        win = ProtonSavesWindow(self)
        win.present()


def main():
    app = ProtonSavesApp()
    return app.run(None)


if __name__ == "__main__":
    main()
