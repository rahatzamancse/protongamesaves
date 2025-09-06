use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow, MessageDialog};
use gtk::{Button, glib, gdk, Align, FileDialog, Window, gio, Box, Orientation, Label, Image};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;

pub struct WelcomeDialog {
    dialog: PreferencesWindow,
    config: Rc<RefCell<Config>>,
    on_complete: Rc<RefCell<Option<std::boxed::Box<dyn FnOnce() + 'static>>>>,
}

impl WelcomeDialog {
    pub fn new<F>(parent: Option<&adw::Application>, config: Rc<RefCell<Config>>, on_complete: F) -> Self 
    where 
        F: FnOnce() + 'static,
    {
        let dialog = PreferencesWindow::builder()
            .title("Welcome to Proton Game Saves Manager")
            .modal(true)
            .default_width(600)
            .default_height(500)
            .build();

        if let Some(app) = parent {
            dialog.set_application(Some(app));
        }
            
        let page = PreferencesPage::new();
        dialog.add(&page);

        // Welcome header group
        let welcome_group = PreferencesGroup::builder()
            .title("Welcome!")
            .description("Let's set up your Steam directory to get started")
            .build();
        page.add(&welcome_group);

        // Add a welcome message
        let welcome_box = Box::new(Orientation::Vertical, 12);
        let welcome_label = Label::builder()
            .label("This application helps you manage your Steam Proton game save files.\n\nTo get started, please select your Steam directory.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        welcome_label.add_css_class("dim-label");
        welcome_box.append(&welcome_label);
        
        welcome_group.add(&welcome_box);

        // Steam directory selection group
        let steam_group = PreferencesGroup::builder()
            .title("Steam Directory")
            .description("Select your Steam installation directory")
            .build();
        page.add(&steam_group);
            
        let steam_path_text = {
            let config_borrow = config.borrow();
            config_borrow.steam_path().to_string_lossy().to_string()
        };
        
        let path_row = ActionRow::builder()
            .title("Steam Directory")
            .subtitle(&steam_path_text)
            .build();
        let browse_button = Button::with_label("Browse");
        browse_button.set_valign(Align::Center);
        path_row.add_suffix(&browse_button);
        
        let dialog_clone = dialog.clone();
        let config_clone = config.clone();
        let path_row_clone = path_row.clone();
        browse_button.connect_clicked(move |_| {
            let config_clone_inner = config_clone.clone();
            let path_row_clone_inner = path_row_clone.clone();
            let parent_window = dialog_clone.clone().upcast::<Window>();
            glib::MainContext::default().spawn_local(async move {
                Self::show_steam_folder_chooser_async(parent_window, config_clone_inner, path_row_clone_inner).await;
            });
        });
        steam_group.add(&path_row);

        // Flatpak permissions group - only show if running in Flatpak
        if Self::is_running_in_flatpak() {
            let flatpak_group = PreferencesGroup::builder()
                .title("Flatpak Permissions")
                .description("Since you're using the Flatpak version, you need to grant filesystem access")
                .build();
            page.add(&flatpak_group);

            // Warning message
            let warning_box = Box::new(Orientation::Vertical, 8);
            let warning_icon = Image::from_icon_name("dialog-warning-symbolic");
            warning_icon.set_icon_size(gtk::IconSize::Large);
            warning_icon.add_css_class("warning");
            warning_box.append(&warning_icon);

            let warning_label = Label::builder()
                .label("⚠️ Important: Flatpak Permission Required")
                .wrap(true)
                .justify(gtk::Justification::Center)
                .build();
            warning_label.add_css_class("heading");
            warning_box.append(&warning_label);

            let info_label = Label::builder()
                .label("Flatpak applications run in a sandbox and need explicit permission to access your Steam directory.\n\nPlease run the following commands in a terminal to grant the necessary permissions:")
                .wrap(true)
                .justify(gtk::Justification::Left)
                .build();
            warning_box.append(&info_label);

            flatpak_group.add(&warning_box);

            // Command instructions
            let cmd_row1 = ActionRow::builder()
                .title("Grant filesystem access")
                .subtitle("flatpak override --user --filesystem=home io.github.rahatzamancse.ProtonGameSaves")
                .build();
            let copy_btn1 = Button::with_label("Copy");
            copy_btn1.set_valign(Align::Center);
            let cmd1 = "flatpak override --user --filesystem=home io.github.rahatzamancse.ProtonGameSaves";
            copy_btn1.connect_clicked(glib::clone!(@strong cmd1 => move |_| {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                clipboard.set_text(&cmd1);
            }));
            cmd_row1.add_suffix(&copy_btn1);
            flatpak_group.add(&cmd_row1);

            let cmd_row2 = ActionRow::builder()
                .title("Grant network access (for manifest downloads)")
                .subtitle("flatpak override --user --share=network io.github.rahatzamancse.ProtonGameSaves")
                .build();
            let copy_btn2 = Button::with_label("Copy");
            copy_btn2.set_valign(Align::Center);
            let cmd2 = "flatpak override --user --share=network io.github.rahatzamancse.ProtonGameSaves";
            copy_btn2.connect_clicked(glib::clone!(@strong cmd2 => move |_| {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                clipboard.set_text(&cmd2);
            }));
            cmd_row2.add_suffix(&copy_btn2);
            flatpak_group.add(&cmd_row2);

            let restart_label = Label::builder()
                .label("After running these commands, you may need to restart the application.")
                .wrap(true)
                .justify(gtk::Justification::Center)
                .build();
            restart_label.add_css_class("dim-label");
            let restart_box = Box::new(Orientation::Vertical, 6);
            restart_box.append(&restart_label);
            flatpak_group.add(&restart_box);
        }

        // Completion group
        let complete_group = PreferencesGroup::builder()
            .title("Ready to Go")
            .description("Click 'Get Started' when you're ready")
            .build();
        page.add(&complete_group);

        let complete_row = ActionRow::builder()
            .title("Complete Setup")
            .subtitle("Save configuration and start using the application")
            .build();
        let complete_button = Button::builder()
            .label("Get Started")
            .css_classes(vec!["suggested-action".to_string()])
            .valign(Align::Center)
            .build();
        
        complete_row.add_suffix(&complete_button);
        complete_group.add(&complete_row);

        // Handle completion
        let config_complete = config.clone();
        let dialog_complete = dialog.clone();
        let on_complete_callback = Rc::new(RefCell::new(Some(std::boxed::Box::new(on_complete) as std::boxed::Box<dyn FnOnce() + 'static>)));
        let on_complete_for_click = on_complete_callback.clone();
        complete_button.connect_clicked(move |_| {
            // Mark first run as complete
            if let Err(e) = config_complete.borrow_mut().mark_first_run_complete() {
                eprintln!("Failed to save configuration: {}", e);
                Self::show_error_dialog(&dialog_complete.clone().upcast::<Window>(), 
                    "Configuration Error", 
                    &format!("Failed to save configuration: {}", e));
                return;
            }
            
            // Close the dialog
            dialog_complete.close();
            
            // Execute the callback
            if let Some(callback) = on_complete_for_click.borrow_mut().take() {
                callback();
            }
        });

        Self {
            dialog,
            config,
            on_complete: on_complete_callback,
        }
    }
    
    pub fn present(&self) {
        self.dialog.present();
    }

    fn is_running_in_flatpak() -> bool {
        // Check for common Flatpak environment indicators
        std::env::var("FLATPAK_ID").is_ok() || 
        std::env::var("FLATPAK_DEST").is_ok() ||
        std::path::Path::new("/.flatpak-info").exists()
    }
    
    async fn show_steam_folder_chooser_async(parent: Window, config: Rc<RefCell<Config>>, row: ActionRow) {
        let file_dialog = FileDialog::new();
        file_dialog.set_title("Select Steam Directory");

        match file_dialog.select_folder_future(Some(&parent)).await {
            Ok(folder) => {
                if let Some(path) = folder.path() {
                    println!("Selected folder: {}", path.display());
                    if let Err(e) = config.borrow_mut().set_steam_path(path.clone()) {
                        eprintln!("Error setting steam path: {}", e);
                        Self::show_error_dialog(&parent, "Error Setting Path", 
                            &format!("Failed to set Steam path: {}", e));
                    } else {
                        row.set_subtitle(&path.to_string_lossy());
                    }
                }
            },
            Err(e) => {
                if e.kind::<gio::IOErrorEnum>() == Some(gio::IOErrorEnum::Cancelled) {
                    println!("Folder selection cancelled.");
                } else {
                    eprintln!("Error selecting folder: {}", e);
                    Self::show_error_dialog(&parent, "Selection Error", 
                        &format!("Failed to select folder: {}", e));
                }
            }
        }
    }
    
    fn show_error_dialog(parent: &Window, title: &str, message: &str) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .heading(title)
            .body(message)
            .build();
        dialog.add_response("ok", "OK");
        dialog.present();
    }
}
