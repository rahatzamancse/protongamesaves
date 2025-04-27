use adw::prelude::*;
use adw::{ActionRow, ApplicationWindow, HeaderBar, PreferencesGroup, PreferencesPage, PreferencesWindow, WindowTitle};
use gtk::{gio, glib, Box, Button, FileChooserAction, FileChooserDialog, Orientation, ResponseType, Stack, StackSwitcher};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;
use crate::ui::compatdata_page::CompatDataPage;
use crate::ui::games_page::GamesPage;
use crate::ui::settings::SettingsDialog;

pub struct ProtonSavesWindow {
    window: ApplicationWindow,
    config: Rc<RefCell<Config>>,
    compat_page: Rc<CompatDataPage>,
    games_page: Rc<RefCell<GamesPage>>,
}

impl ProtonSavesWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Create config
        let config = Rc::new(RefCell::new(Config::new()));
        
        // Create the main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Proton Game Saves Manager")
            .default_width(900)
            .default_height(700)
            .build();
            
        // Create header bar
        let header_bar = HeaderBar::new();
        
        // Add Refresh button to HeaderBar start
        let refresh_button = Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh Lists"));
        header_bar.pack_start(&refresh_button);

        // Create StackSwitcher for page navigation
        let stack_switcher = StackSwitcher::new();
        header_bar.set_title_widget(Some(&stack_switcher));
        
        // Create menu button
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        
        // Create menu
        let menu = gio::Menu::new();
        menu.append(Some("About"), Some("app.about"));
        menu.append(Some("Settings"), Some("app.settings"));
        menu.append(Some("Quit"), Some("app.quit"));
        
        menu_button.set_menu_model(Some(&menu));
        header_bar.pack_end(&menu_button);
        
        // Create main box
        let main_box = Box::new(Orientation::Vertical, 0);
        main_box.append(&header_bar);
        
        // Create the Stack to hold pages
        let stack = Stack::new();
        stack.set_vexpand(true);
        main_box.append(&stack);
        
        // Set the main box as the content of the window
        window.set_content(Some(&main_box));

        // Create the CompatDataPage
        let compat_page = Rc::new(CompatDataPage::new(window.clone(), config.clone()));
        stack.add_titled(compat_page.widget(), Some("compatdata"), "Compatdata");

        // Create the GamesPage (using RefCell for interior mutability needed for update_manifest)
        let games_page = Rc::new(RefCell::new(GamesPage::new(config.clone())));
        stack.add_titled(games_page.borrow().widget(), Some("games"), "Games");
        
        // Connect StackSwitcher
        stack_switcher.set_stack(Some(&stack));

        // Initial manifest load for GamesPage
        // Moved initial populate call to after connect_clicked setup
        // games_page.borrow_mut().update_manifest(); 

        // Refresh button action - Refactored
        let compat_page_clone = compat_page.clone();
        let games_page_clone = games_page.clone();
        let window_clone = window.clone(); // Clone window for error dialog
        refresh_button.connect_clicked(move |_| {
            println!("Refresh button clicked.");
            // Scan compatdata first
            match compat_page_clone.scan_compatdata() {
                Ok(prefixes) => {
                    println!("Compatdata scan successful, found {} prefixes.", prefixes.len());
                    // Update CompatDataPage UI
                    compat_page_clone.update_listbox(&prefixes);
                    
                    // Populate GamesPage with the scanned data
                    // Need to update populate_games signature to accept Vec<PrefixData>
                    games_page_clone.borrow().populate_games(&prefixes); 
                }
                Err(e) => {
                    eprintln!("Error scanning compatdata: {}", e);
                    // Show error dialog using the window's helper method if possible
                    // Or create a new one.
                    let error_dialog = gtk::MessageDialog::builder()
                        .transient_for(&window_clone)
                        .modal(true)
                        .buttons(gtk::ButtonsType::Ok)
                        .message_type(gtk::MessageType::Error)
                        .text("Error Scanning Compatdata")
                        .secondary_text(&format!("{}", e))
                        .build();
                    error_dialog.connect_response(|dialog, _| dialog.destroy());
                    error_dialog.present();
                }
            }
        });
        
        // Initial manifest load happens here now
        games_page.borrow_mut().update_manifest(); 
        // Trigger initial refresh to populate lists on startup
        refresh_button.emit_clicked(); 

        // Create the application actions
        Self::create_actions(app, window.clone(), config.clone(), compat_page.clone(), games_page.clone(), refresh_button.clone());
        
        Self {
            window,
            config,
            compat_page,
            games_page,
        }
    }
    
    pub fn present(&self) {
        self.window.present();
    }
    
    fn create_actions(app: &adw::Application, window: ApplicationWindow, config: Rc<RefCell<Config>>, compat_page: Rc<CompatDataPage>, games_page: Rc<RefCell<GamesPage>>, refresh_button: Button) {
        // Quit action
        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(glib::clone!(@weak app => move |_, _| {
            app.quit();
        }));
        app.add_action(&quit_action);
        
        // About action
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(glib::clone!(@weak window => move |_, _| {
            Self::show_about_dialog(&window);
        }));
        app.add_action(&about_action);
        
        // Settings action
        let settings_action = gio::SimpleAction::new("settings", None);
        // Clone Rc for the games_page to be used in the outer closure
        let games_page_for_settings = games_page.clone(); 
        // Clone refresh_button for the outer closure (and potentially inner)
        let refresh_button_clone = refresh_button.clone(); 
        settings_action.connect_activate(glib::clone!(@weak window, @strong config, @strong games_page_for_settings, @strong refresh_button_clone => move |_, _| {
            // Clone again for the inner FnMut closure
            let games_page_for_callback = games_page_for_settings.clone();
            // Clone refresh button again for inner closure
            let refresh_button_for_callback = refresh_button_clone.clone(); 
            // Create the callback closure 
            let on_update_callback = Rc::new(RefCell::new(move || { 
                println!("Settings updated, triggering manifest refresh...");
                games_page_for_callback.borrow_mut().update_manifest();
                println!("Manifest updated via settings, triggering full refresh...");
                // Now trigger the main refresh button
                refresh_button_for_callback.emit_clicked(); 
                // TODO: Consider if only populating games is needed vs full refresh
            }));
            
            // Pass the callback to SettingsDialog::new
            let dialog = SettingsDialog::new(window.clone(), config.clone(), on_update_callback);
            dialog.present(); 
        }));
        app.add_action(&settings_action);
    }
    
    fn show_about_dialog(window: &ApplicationWindow) {
        let about = adw::AboutWindow::builder()
            .transient_for(window)
            .application_name("Proton Game Saves Manager")
            .version("0.1.0")
            .developer_name("Proton Game Saves Manager Team")
            .license_type(gtk::License::Gpl30)
            .comments("Manage your Steam Proton game save files")
            .website("https://github.com/username/proton-gamesaves")
            .issue_url("https://github.com/username/proton-gamesaves/issues")
            .build();
            
        about.present();
    }
} 