//! Velotype - a block-based Markdown editor built with GPUI.
//!
//! Reads file paths from command-line arguments and opens one GPUI window per
//! file. With no arguments, a single empty window is created.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use gpui::*;

mod app_identity;
mod app_menu;
mod components;
mod config;
mod editor;
mod export;
mod i18n;
mod net;
mod theme;

use app_menu::{init as init_app_menu, open_editor_window, open_file_in_new_window};
use components::init_with_keybindings as init_editor;
use i18n::I18nManager;
use theme::ThemeManager;

fn main() {
    let input_paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let pending_urls: Rc<RefCell<Option<Vec<String>>>> = Rc::new(RefCell::new(None));

    let app = Application::new();
    
    let pending_urls_clone = pending_urls.clone();
    app.on_open_urls(move |urls| {
        // 保存 URL
        *pending_urls_clone.borrow_mut() = Some(urls);
    });

    let pending_urls_clone2 = pending_urls.clone();
    app.on_reopen(move |cx: &mut App| {
        // 检查是否有待处理的 URL
        if let Some(urls) = pending_urls_clone2.borrow_mut().take() {
            for url in urls {
                if let Ok(url_obj) = url::Url::parse(&url) {
                    if let Ok(path) = url_obj.to_file_path() {
                        if let Err(err) = open_file_in_new_window(cx, &path) {
                            eprintln!("Failed to open file {}: {}", path.display(), err);
                        }
                    }
                }
            }
        }
    });

    app.run(move |cx: &mut App| {
        let preferences = config::load_or_create_app_preferences().unwrap_or_else(|err| {
            eprintln!("failed to initialize app preferences: {err}");
            Default::default()
        });
        I18nManager::init_with_language_id(cx, &preferences.default_language_id);
        ThemeManager::init_with_theme_id(cx, &preferences.default_theme_id);
        net::install_http_client(cx);
        init_editor(cx, &preferences.keybindings);
        init_app_menu(cx);

        // 处理命令行参数或启动时传入的 URL
        let mut paths_to_open = input_paths.clone();
        
        // 检查是否有待处理的 URL（启动时传入的）
        if let Some(urls) = pending_urls.borrow_mut().take() {
            for url in urls {
                if let Ok(url_obj) = url::Url::parse(&url) {
                    if let Ok(path) = url_obj.to_file_path() {
                        paths_to_open.push(path);
                    }
                }
            }
        }

        if paths_to_open.is_empty() {
            if preferences.startup_open == config::StartupOpenPreference::LastOpenedFile {
                if let Some(path) = config::first_existing_recent_markdown_file() {
                    if let Err(err) = open_file_in_new_window(cx, &path) {
                        eprintln!("failed to read last opened file '{}': {}", path.display(), err);
                    } else {
                        return;
                    }
                }
            }
            open_editor_window(cx, String::new(), None);
            return;
        }

        for path in &paths_to_open {
            if let Err(err) = open_file_in_new_window(cx, path) {
                eprintln!("Failed to open file {}: {}", path.display(), err);
            }
        }
        app_menu::install_menus(cx);
        cx.refresh_windows();
    });
}
