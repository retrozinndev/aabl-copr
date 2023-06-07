use relm4::{
    prelude::*,
    component::*,
    actions::*,
    MessageBroker
};

use gtk::prelude::*;
use adw::prelude::*;

use gtk::glib::clone;

mod repair_game;
mod download_wine;
mod create_prefix;
mod install_mfc140;
mod install_corefonts;
mod download_diff;
mod launch;

use anime_launcher_sdk::components::loader::ComponentsLoader;

use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::pgr::config::Config;

use anime_launcher_sdk::pgr::config::schema::launcher::LauncherStyle;

use anime_launcher_sdk::pgr::states::*;
use anime_launcher_sdk::pgr::consts::*;

use crate::*;
use crate::i18n::*;
use crate::ui::components::*;

use super::preferences::main::*;
use super::about::*;

relm4::new_action_group!(WindowActionGroup, "win");

relm4::new_stateless_action!(LauncherFolder, WindowActionGroup, "launcher_folder");
relm4::new_stateless_action!(GameFolder, WindowActionGroup, "game_folder");
relm4::new_stateless_action!(ConfigFile, WindowActionGroup, "config_file");
relm4::new_stateless_action!(DebugFile, WindowActionGroup, "debug_file");
// relm4::new_stateless_action!(WishUrl, WindowActionGroup, "wish_url");

relm4::new_stateless_action!(About, WindowActionGroup, "about");

pub static mut MAIN_WINDOW: Option<adw::ApplicationWindow> = None;
pub static mut PREFERENCES_WINDOW: Option<AsyncController<PreferencesApp>> = None;
pub static mut ABOUT_DIALOG: Option<Controller<AboutDialog>> = None;

pub struct App {
    progress_bar: AsyncController<ProgressBar>,

    toast_overlay: adw::ToastOverlay,

    loading: Option<Option<String>>,
    style: LauncherStyle,
    state: Option<LauncherState>,

    downloading: bool,
    disabled_buttons: bool
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateLauncherState {
        /// Perform action when game or voice downloading is required
        /// Needed for chained executions (e.g. update one voice after another)
        perform_on_download_needed: bool,

        /// Show status gathering progress page
        show_status_page: bool
    },

    /// Supposed to be called automatically on app's run when the latest game version
    /// was retrieved from the API
    SetGameDiff(Option<VersionDiff>),

    /// Supposed to be called automatically on app's run when the launcher state was chosen
    SetLauncherState(Option<LauncherState>),

    SetLauncherStyle(LauncherStyle),
    SetLoadingStatus(Option<Option<String>>),

    SetDownloading(bool),
    DisableButtons(bool),

    OpenPreferences,
    RepairGame,

    PerformAction,

    HideWindow,
    ShowWindow,

    Toast {
        title: String,
        description: Option<String>
    }
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    menu! {
        main_menu: {
            section! {
                &tr("launcher-folder") => LauncherFolder,
                &tr("game-folder") => GameFolder,
                &tr("config-file") => ConfigFile,
                &tr("debug-file") => DebugFile,
            },

            /*section! {
                &tr("wish-url") => WishUrl
            },*/

            section! {
                &tr("about") => About
            }
        }
    }

    view! {
        main_window = adw::ApplicationWindow {
            set_icon_name: Some(APP_ID),

            #[watch]
            set_default_size: (
                match model.style {
                    LauncherStyle::Modern => 900,
                    LauncherStyle::Classic => 1094 // (w = 1280 / 730 * h, where 1280x730 is default background picture resolution)
                },
                match model.style {
                    LauncherStyle::Modern => 600,
                    LauncherStyle::Classic => 624
                }
            ),

            #[watch]
            set_css_classes: &{
                let mut classes = vec!["background", "csd"];

                if APP_DEBUG {
                    classes.push("devel");
                }

                match model.style {
                    LauncherStyle::Modern => (),
                    LauncherStyle::Classic => {
                        if model.loading.is_none() {
                            classes.push("classic-style");
                        }
                    }
                }

                classes
            },

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::HeaderBar {
                        #[watch]
                        set_css_classes: match model.style {
                            LauncherStyle::Modern => &[""],
                            LauncherStyle::Classic => &["flat"]
                        },

                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            #[watch]
                            set_title: match model.style {
                                LauncherStyle::Modern => "An Anime Borb Launcher",
                                LauncherStyle::Classic => ""
                            }
                        },

                        pack_end = &gtk::MenuButton {
                            set_icon_name: "open-menu-symbolic",
                            set_menu_model: Some(&main_menu)
                        }
                    },

                    adw::StatusPage {
                        set_title: &tr("loading-data"),
                        set_icon_name: Some(APP_ID),
                        set_vexpand: true,

                        #[watch]
                        set_description: match &model.loading {
                            Some(Some(desc)) => Some(desc),
                            Some(None) | None => None
                        },

                        #[watch]
                        set_visible: model.loading.is_some()
                    },

                    adw::PreferencesPage {
                        #[watch]
                        set_visible: model.loading.is_none(),

                        add = &adw::PreferencesGroup {
                            set_margin_top: 48,

                            #[watch]
                            set_visible: model.style == LauncherStyle::Modern,

                            gtk::Picture {
                                set_resource: Some("/org/app/images/icon.png"),
                                set_vexpand: true,
                                set_content_fit: gtk::ContentFit::ScaleDown
                            },

                            gtk::Label {
                                set_label: "An Anime Borb Launcher",
                                set_margin_top: 32,
                                add_css_class: "title-1"
                            }
                        },

                        add = &adw::PreferencesGroup {
                            #[watch]
                            set_valign: match model.style {
                                LauncherStyle::Modern => gtk::Align::Center,
                                LauncherStyle::Classic => gtk::Align::End
                            },

                            #[watch]
                            set_width_request: match model.style {
                                LauncherStyle::Modern => -1,
                                LauncherStyle::Classic => 800
                            },

                            #[watch]
                            set_visible: model.downloading,

                            set_vexpand: true,
                            set_margin_top: 48,
                            set_margin_bottom: 48,

                            add = model.progress_bar.widget(),
                        },

                        add = &adw::PreferencesGroup {
                            #[watch]
                            set_valign: match model.style {
                                LauncherStyle::Modern => gtk::Align::Center,
                                LauncherStyle::Classic => gtk::Align::End
                            },

                            #[watch]
                            set_width_request: match model.style {
                                LauncherStyle::Modern => -1,
                                LauncherStyle::Classic => 800
                            },

                            #[watch]
                            set_visible: !model.downloading,

                            #[watch]
                            set_margin_bottom: match model.style {
                                LauncherStyle::Modern => 48,
                                LauncherStyle::Classic => 0
                            },

                            set_vexpand: true,

                            gtk::Box {
                                #[watch]
                                set_halign: match model.style {
                                    LauncherStyle::Modern => gtk::Align::Center,
                                    LauncherStyle::Classic => gtk::Align::End
                                },

                                #[watch]
                                set_height_request: match model.style {
                                    LauncherStyle::Modern => -1,
                                    LauncherStyle::Classic => 40
                                },

                                set_margin_top: 64,
                                set_spacing: 8,

                                adw::Bin {
                                    set_css_classes: &["background", "round-bin"],

                                    gtk::Button {
                                        adw::ButtonContent {
                                            #[watch]
                                            set_icon_name: match &model.state {
                                                Some(LauncherState::Launch) => "media-playback-start-symbolic",

                                                Some(LauncherState::WineNotInstalled) |
                                                Some(LauncherState::PrefixNotExists) |

                                                Some(LauncherState::Mfc140NotInstalled) |
                                                Some(LauncherState::CorefontsNotInstalled(_)) |

                                                Some(LauncherState::GameUpdateAvailable(_)) |
                                                Some(LauncherState::GameNotInstalled(_)) => "document-save-symbolic",

                                                None => "window-close-symbolic"
                                            },

                                            #[watch]
                                            set_label: &match &model.state {
                                                Some(LauncherState::Launch) => tr("launch"),

                                                Some(LauncherState::WineNotInstalled) => tr("download-wine"),
                                                Some(LauncherState::PrefixNotExists)  => tr("create-prefix"),

                                                // TODO: add localization
                                                Some(LauncherState::Mfc140NotInstalled) => String::from("Install mfc140"),
                                                Some(LauncherState::CorefontsNotInstalled(_)) => String::from("Install corefonts"),

                                                Some(LauncherState::GameUpdateAvailable(diff)) => {
                                                    match (Config::get(), diff.file_name()) {
                                                        (Ok(config), Some(filename)) => {
                                                            let temp = config.launcher.temp.unwrap_or_else(std::env::temp_dir);

                                                            if temp.join(filename).exists() {
                                                                tr("resume")
                                                            }

                                                            else {
                                                                tr("update")
                                                            }
                                                        }

                                                        _ => tr("update")
                                                    }
                                                },

                                                Some(LauncherState::GameNotInstalled(_)) => tr("download"),

                                                None => String::from("...")
                                            }
                                        },

                                        #[watch]
                                        set_sensitive: !model.disabled_buttons && model.state.is_some(),

                                        #[watch]
                                        set_css_classes: match &model.state {
                                            Some(_) => &["suggested-action", "pill"],
                                            None => &["pill"]
                                        },

                                        set_hexpand: false,
                                        set_width_request: 200,

                                        connect_clicked => AppMsg::PerformAction
                                    }
                                },

                                adw::Bin {
                                    set_css_classes: &["background", "round-bin"],

                                    gtk::Button {
                                        #[watch]
                                        set_sensitive: !model.disabled_buttons,

                                        set_width_request: 44,

                                        add_css_class: "circular",
                                        set_icon_name: "emblem-system-symbolic",

                                        connect_clicked => AppMsg::OpenPreferences
                                    }
                                }
                            }
                        }
                    }
                }
            },

            connect_close_request[sender] => move |_| {
                if let Err(err) = Config::flush() {
                    sender.input(AppMsg::Toast {
                        title: tr("config-update-error"),
                        description: Some(err.to_string())
                    });
                }

                gtk::Inhibit::default()
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        tracing::info!("Initializing main window");

        let model = App {
            progress_bar: ProgressBar::builder()
                .launch(ProgressBarInit {
                    caption: None,
                    display_progress: true,
                    display_fraction: true,
                    visible: true
                })
                .detach(),

            toast_overlay: adw::ToastOverlay::new(),

            loading: Some(None),
            style: CONFIG.launcher.style,
            state: None,

            downloading: false,
            disabled_buttons: false
        };

        model.progress_bar.widget().set_halign(gtk::Align::Center);
        model.progress_bar.widget().set_width_request(360);

        let toast_overlay = &model.toast_overlay;

        let widgets = view_output!();

        let about_dialog_broker: MessageBroker<AboutDialogMsg> = MessageBroker::new();

        unsafe {
            MAIN_WINDOW = Some(widgets.main_window.clone());

            PREFERENCES_WINDOW = Some(PreferencesApp::builder()
                .launch(widgets.main_window.clone().into())
                .forward(sender.input_sender(), std::convert::identity));

            ABOUT_DIALOG = Some(AboutDialog::builder()
                .transient_for(widgets.main_window.clone())
                .launch_with_broker((), &about_dialog_broker)
                .detach());
        }

        let mut group = RelmActionGroup::<WindowActionGroup>::new();

        // TODO: reduce code somehow

        group.add_action::<LauncherFolder>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Err(err) = open::that(LAUNCHER_FOLDER.as_path()) {
                sender.input(AppMsg::Toast {
                    title: tr("launcher-folder-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open launcher folder: {err}");
            }
        })));

        group.add_action::<GameFolder>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            let path = match Config::get() {
                Ok(config) => config.game.path,
                Err(_) => CONFIG.game.path.clone(),
            };

            if let Err(err) = open::that(path) {
                sender.input(AppMsg::Toast {
                    title: tr("game-folder-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open game folder: {err}");
            }
        })));

        group.add_action::<ConfigFile>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Ok(file) = config_file() {
                if let Err(err) = open::that(file) {
                    sender.input(AppMsg::Toast {
                        title: tr("config-file-opening-error"),
                        description: Some(err.to_string())
                    });

                    tracing::error!("Failed to open config file: {err}");
                }
            }
        })));

        group.add_action::<DebugFile>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Err(err) = open::that(crate::DEBUG_FILE.as_os_str()) {
                sender.input(AppMsg::Toast {
                    title: tr("debug-file-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open debug file: {err}");
            }
        })));

        /*group.add_action::<WishUrl>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            std::thread::spawn(clone!(@strong sender => move || {
                let config = Config::get().unwrap_or_else(|_| CONFIG.clone());

                let web_cache = config.game.path
                    .join(DATA_FOLDER_NAME)
                    .join("webCaches/Cache/Cache_Data/data_2");

                if !web_cache.exists() {
                    tracing::error!("Couldn't find wishes URL: cache file doesn't exist");

                    sender.input(AppMsg::Toast {
                        title: tr("wish-url-search-failed"),
                        description: None
                    });
                }

                else {
                    match std::fs::read(&web_cache) {
                        Ok(web_cache) => {
                            let web_cache = String::from_utf8_lossy(&web_cache);

                            // https://webstatic-sea.[ho-yo-ver-se].com/[ge-nsh-in]/event/e20190909gacha-v2/index.html?......
                            if let Some(url) = web_cache.lines().rev().find(|line| line.contains("gacha-v2/index.html")) {
                                let url_begin_pos = url.find("https://").unwrap();
                                let url_end_pos = url_begin_pos + url[url_begin_pos..].find("\0\0\0\0").unwrap();

                                if let Err(err) = open::that(format!("{}#/log", &url[url_begin_pos..url_end_pos])) {
                                    tracing::error!("Failed to open wishes URL: {err}");
    
                                    sender.input(AppMsg::Toast {
                                        title: tr("wish-url-opening-error"),
                                        description: Some(err.to_string())
                                    });
                                }
                            }

                            else {
                                tracing::error!("Couldn't find wishes URL: no url found");

                                sender.input(AppMsg::Toast {
                                    title: tr("wish-url-search-failed"),
                                    description: None
                                });
                            }
                        }

                        Err(err) => {
                            tracing::error!("Couldn't find wishes URL: failed to open cache file: {err}");

                            sender.input(AppMsg::Toast {
                                title: tr("wish-url-search-failed"),
                                description: Some(err.to_string())
                            });
                        }
                    }
                }
            }));
        })));*/

        group.add_action::<About>(RelmAction::new_stateless(move |_| {
            about_dialog_broker.send(AboutDialogMsg::Show);
        }));

        widgets.main_window.insert_action_group("win", Some(&group.into_action_group()));

        tracing::info!("Main window initialized");

        let download_picture = model.style == LauncherStyle::Classic && !KEEP_BACKGROUND_FILE.exists();

        // Initialize some heavy tasks
        std::thread::spawn(move || {
            tracing::info!("Initializing heavy tasks");

            // Download background picture if needed

            if download_picture {
                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("downloading-background-picture")))));

                if let Err(err) = crate::background::download_background() {
                    tracing::error!("Failed to download background picture: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("background-downloading-failed"),
                        description: Some(err.to_string())
                    });
                }
            }

            // Update components index

            sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("updating-components-index")))));

            let components = ComponentsLoader::new(&CONFIG.components.path);

            match components.is_sync(&CONFIG.components.servers) {
                Ok(Some(_)) => (),

                Ok(None) => {
                    for host in &CONFIG.components.servers {
                        match components.sync(host) {
                            Ok(changes) => {
                                sender.input(AppMsg::Toast {
                                    title: tr("components-index-updated"),
                                    description: if changes.is_empty() {
                                        None
                                    } else {
                                        Some(changes.into_iter()
                                            .map(|line| format!("- {line}"))
                                            .collect::<Vec<_>>()
                                            .join("\n"))
                                    }
                                });

                                break;
                            }

                            Err(err) => {
                                tracing::error!("Failed to sync components index");

                                sender.input(AppMsg::Toast {
                                    title: tr("components-index-sync-failed"),
                                    description: Some(err.to_string())
                                });
                            }
                        }
                    }
                }

                Err(err) => {
                    tracing::error!("Failed to verify that components index synced");

                    sender.input(AppMsg::Toast {
                        title: tr("components-index-verify-failed"),
                        description: Some(err.to_string())
                    });
                }
            }

            // Update initial game version status

            sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-game-version")))));

            sender.input(AppMsg::SetGameDiff(match GAME.try_get_diff() {
                Ok(diff) => Some(diff),
                Err(err) => {
                    tracing::error!("Failed to find game diff: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("game-diff-finding-error"),
                        description: Some(err.to_string())
                    });

                    None
                }
            }));

            tracing::info!("Updated game version status");

            // Update launcher state
            sender.input(AppMsg::UpdateLauncherState {
                perform_on_download_needed: false,
                show_status_page: true
            });

            // Mark app as loaded
            unsafe {
                crate::READY = true;
            }

            tracing::info!("App is ready");
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            // TODO: make function from this message like with toast
            AppMsg::UpdateLauncherState { perform_on_download_needed, show_status_page } => {
                if show_status_page {
                    sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-launcher-state")))));
                } else {
                    self.disabled_buttons = true;
                }

                let updater = clone!(@strong sender => move |state| {
                    if show_status_page {
                        match state {
                            StateUpdating::Components => {
                                // TODO: add localizations
                                sender.input(AppMsg::SetLoadingStatus(Some(Some(String::from("Loading launcher state: checking components")))));
                            }

                            StateUpdating::Game => {
                                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-launcher-state--game")))));
                            }
                        }
                    }
                });

                let state = match LauncherState::get_from_config(updater) {
                    Ok(state) => Some(state),
                    Err(err) => {
                        tracing::error!("Failed to update launcher state: {err}");

                        self.toast(tr("launcher-state-updating-error"), Some(err.to_string()));
    
                        None
                    }
                };

                sender.input(AppMsg::SetLauncherState(state.clone()));

                if show_status_page {
                    sender.input(AppMsg::SetLoadingStatus(None));
                } else {
                    self.disabled_buttons = false;
                }

                if let Some(state) = state {
                    match state {
                        LauncherState::GameUpdateAvailable(_) |
                        LauncherState::GameNotInstalled(_) if perform_on_download_needed => {
                            sender.input(AppMsg::PerformAction);
                        }

                        _ => ()
                    }
                }
            }

            #[allow(unused_must_use)]
            AppMsg::SetGameDiff(diff) => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().sender().send(PreferencesAppMsg::SetGameDiff(diff));
            }

            AppMsg::SetLauncherState(state) => {
                self.state = state;
            }

            AppMsg::SetLoadingStatus(status) => {
                self.loading = status;
            }

            AppMsg::SetLauncherStyle(style) => {
                self.style = style;
            }

            AppMsg::SetDownloading(state) => {
                self.downloading = state;
            }

            AppMsg::DisableButtons(state) => {
                self.disabled_buttons = state;
            }

            AppMsg::OpenPreferences => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().widget().present();
            }

            AppMsg::RepairGame => repair_game::repair_game(sender, self.progress_bar.sender().to_owned()),

            AppMsg::PerformAction => unsafe {
                match self.state.as_ref().unwrap_unchecked() {
                    LauncherState::Launch => launch::launch(sender),

                    LauncherState::WineNotInstalled => download_wine::download_wine(sender, self.progress_bar.sender().to_owned()),
                    LauncherState::PrefixNotExists => create_prefix::create_prefix(sender),

                    LauncherState::Mfc140NotInstalled => install_mfc140::install_mfc140(sender),
                    LauncherState::CorefontsNotInstalled(fonts) =>
                        install_corefonts::install_corefonts(sender, self.progress_bar.sender().to_owned(), fonts.clone()),

                    LauncherState::GameUpdateAvailable(diff) |
                    LauncherState::GameNotInstalled(diff) =>
                        download_diff::download_diff(sender, self.progress_bar.sender().to_owned(), diff.to_owned())
                }
            }

            AppMsg::HideWindow => unsafe {
                MAIN_WINDOW.as_ref().unwrap_unchecked().set_visible(false);
            }

            AppMsg::ShowWindow => unsafe {
                MAIN_WINDOW.as_ref().unwrap_unchecked().present();
            }

            AppMsg::Toast { title, description } => self.toast(title, description)
        }
    }
}

impl App {
    pub fn toast<T: AsRef<str>>(&mut self, title: T, description: Option<T>) {
        let toast = adw::Toast::new(title.as_ref());

        toast.set_timeout(4);

        if let Some(description) = description {
            toast.set_button_label(Some(&tr("details")));

            let dialog = adw::MessageDialog::new(
                Some(unsafe { MAIN_WINDOW.as_ref().unwrap_unchecked() }),
                Some(title.as_ref()),
                Some(description.as_ref())
            );

            dialog.add_response("close", &tr("close"));
            dialog.add_response("save", &tr("save"));

            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

            dialog.connect_response(Some("save"), |_, _| {
                if let Err(err) = open::that(crate::DEBUG_FILE.as_os_str()) {
                    tracing::error!("Failed to open debug file: {err}");
                }
            });

            toast.connect_button_clicked(move |_| {
                dialog.present();
            });
        }

        self.toast_overlay.add_toast(toast);
    }
}
