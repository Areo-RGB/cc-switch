mod agy_config;
mod app_config;
mod auto_launch;
mod claude_mcp;
mod codex_config;
mod codex_rollout_file;
mod codex_state_db;
mod commands;
mod config;
mod database;
mod deeplink;
mod dsh_config;
mod error;
mod gemini_config;
mod gemini_mcp;
mod grok_config;
pub mod hermes_config;
mod jsonc_document;
mod kimi_config;
mod kimi_session;
mod mcode_config;
mod mcp;
mod omp_config;
mod openclaw_config;
mod opencode_config;
mod panic_hook;
mod pi_config;
mod prompt;
mod prompt_files;
mod services;
mod session_manager;
mod settings;
mod store;
mod tray;

pub use app_config::{AppType, InstalledSkill, McpApps, McpServer, MultiAppConfig, SkillApps};
pub use codex_config::{
    get_codex_auth_path, get_codex_config_path, read_and_validate_codex_config_text,
};
pub use commands::*;
pub use config::{get_claude_mcp_path, get_claude_settings_path, read_json_file};
pub use database::Database;
pub use error::AppError;
pub use grok_config::get_grok_config_path;
pub use mcp::{
    import_from_claude, import_from_codex, import_from_gemini, import_from_grokbuild,
    remove_server_from_claude, remove_server_from_codex, remove_server_from_gemini,
    remove_server_from_grokbuild, sync_enabled_to_claude, sync_enabled_to_codex,
    sync_enabled_to_gemini, sync_single_server_to_claude, sync_single_server_to_codex,
    sync_single_server_to_gemini, sync_single_server_to_grokbuild,
};
pub use prompt::Prompt;
pub use services::{
    skill::{migrate_skills_to_ssot, ImportSkillSelection},
    ConfigService, McpService, PromptService, SkillService,
};
pub use settings::{update_settings, AppSettings};
pub use store::AppState;

use std::sync::Arc;
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

const MIN_KNOWN_SECRET_LEN: usize = 8;

fn redact_known_secrets_with_min_length(
    text: &str,
    known_secrets: &[String],
    minimum_chars: usize,
) -> String {
    let mut output = text.to_string();
    for secret in known_secrets {
        if secret.chars().count() >= minimum_chars {
            output = output.replace(secret, "[REDACTED]");
        }
    }
    output
}

pub(crate) fn redact_known_secrets(text: &str, known_secrets: &[String]) -> String {
    redact_known_secrets_with_min_length(text, known_secrets, MIN_KNOWN_SECRET_LEN)
}

fn strip_bare_userinfo(input: &str) -> &str {
    let authority_end = input.find('/').unwrap_or(input.len());
    match input[..authority_end].rfind('@') {
        Some(at) => &input[at + 1..],
        None => input,
    }
}

pub(crate) fn redact_url_for_log(url: &str) -> String {
    redact_url_for_log_with_secrets(url, &[])
}

pub(crate) fn redact_url_for_log_with_secrets(url_str: &str, known_secrets: &[String]) -> String {
    let scheme_relative = url_str.starts_with("//");
    let parsed = if scheme_relative {
        url::Url::parse(&format!("https:{url_str}"))
    } else {
        url::Url::parse(url_str)
    };

    let sanitized = match parsed {
        Ok(mut url) if url.has_host() => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            let rendered = url.as_str();
            if scheme_relative {
                rendered
                    .strip_prefix("https:")
                    .unwrap_or(rendered)
                    .to_string()
            } else {
                rendered.to_string()
            }
        }
        _ => strip_bare_userinfo(url_str.split(['?', '#']).next().unwrap_or(url_str)).to_string(),
    };

    redact_known_secrets(&sanitized, known_secrets)
}

fn handle_deeplink_url(app: &tauri::AppHandle, url_str: &str) -> bool {
    if !url_str.starts_with("ccswitch://") {
        return false;
    }

    let pending = match deeplink::parse_deeplink_url(url_str) {
        Ok(request) => deeplink::PendingDeepLink::Import {
            request: Box::new(request),
        },
        Err(error) => deeplink::PendingDeepLink::Error {
            url: url_str.to_string(),
            error: error.to_string(),
        },
    };
    deeplink::set_pending_deeplink(pending);
    let _ = app.emit("deeplink-pending", ());
    tray::show_main_window(app);
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    panic_hook::setup_panic_hook();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            let _ = rustls::crypto::ring::default_provider().install_default();

            let db = Arc::new(Database::init()?);
            if let Err(error) = db.init_default_skill_repos() {
                log::warn!("Failed to initialize default skill repositories: {error}");
            }

            services::webdav_auto_sync::start_worker(Arc::clone(&db), app.handle().clone());
            services::s3_auto_sync::start_worker(Arc::clone(&db), app.handle().clone());

            let app_state = AppState::new(Arc::clone(&db));
            let menu = tray::create_tray_menu(app.handle(), &app_state)?;
            app.manage(app_state);
            app.manage(commands::skill::SkillServiceState(Arc::new(
                SkillService::new(),
            )));

            TrayIconBuilder::with_id(tray::TRAY_ID)
                .tooltip("CC Switch Desk")
                .menu(&menu)
                .on_tray_icon_event(tray::handle_tray_icon_event)
                .on_menu_event(|app, event| tray::handle_tray_menu_event(app, &event.id.0))
                .build(app)?;

            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    if handle_deeplink_url(&app_handle, url.as_str()) {
                        break;
                    }
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    if handle_deeplink_url(app.handle(), url.as_str()) {
                        break;
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_mcp_servers,
            commands::upsert_mcp_server,
            commands::delete_mcp_server,
            commands::toggle_mcp_app,
            commands::import_mcp_from_apps,
            commands::resync_mcp_to_apps,
            commands::get_claude_mcp_status,
            commands::read_claude_mcp_config,
            commands::upsert_claude_mcp_server,
            commands::delete_claude_mcp_server,
            commands::validate_mcp_command,
            commands::get_mcp_config,
            commands::upsert_mcp_server_in_config,
            commands::delete_mcp_server_in_config,
            commands::set_mcp_enabled,
            commands::get_prompts,
            commands::upsert_prompt,
            commands::delete_prompt,
            commands::enable_prompt,
            commands::import_prompt_from_file,
            commands::get_current_prompt_file_content,
            commands::get_prompt_file_location,
            commands::get_pi_prompt_file,
            commands::replace_pi_prompt_file,
            commands::delete_pi_prompt_file,
            commands::list_pi_prompt_templates,
            commands::upsert_pi_prompt_template,
            commands::delete_pi_prompt_template,
            commands::get_installed_skills,
            commands::get_skill_backups,
            commands::delete_skill_backup,
            commands::install_skill_unified,
            commands::uninstall_skill_unified,
            commands::restore_skill_backup,
            commands::toggle_skill_app,
            commands::scan_unmanaged_skills,
            commands::import_skills_from_apps,
            commands::discover_available_skills,
            commands::discover_repo_skills,
            commands::check_skill_updates,
            commands::resync_skills_to_apps,
            commands::update_skill,
            commands::migrate_skill_storage,
            commands::get_cc_switch_skills_dir,
            commands::open_cc_switch_skills_dir,
            commands::open_skill_dir,
            commands::search_skills_sh,
            commands::get_skill_repos,
            commands::add_skill_repo,
            commands::remove_skill_repo,
            commands::install_skills_from_zip,
            commands::get_skills,
            commands::get_skills_for_app,
            commands::install_skill,
            commands::install_skill_for_app,
            commands::uninstall_skill,
            commands::uninstall_skill_for_app,
            commands::set_auto_launch,
            commands::get_auto_launch_status,
            commands::export_config_to_file,
            commands::import_config_from_file,
            commands::save_file_dialog,
            commands::open_file_dialog,
            commands::open_zip_file_dialog,
            commands::webdav_test_connection,
            commands::webdav_sync_upload,
            commands::webdav_sync_download,
            commands::webdav_sync_save_settings,
            commands::webdav_sync_clear_settings,
            commands::webdav_sync_fetch_remote_info,
            commands::s3_test_connection,
            commands::s3_sync_upload,
            commands::s3_sync_download,
            commands::s3_sync_save_settings,
            commands::s3_sync_fetch_remote_info,
            commands::create_db_backup,
            commands::list_db_backups,
            commands::restore_db_backup,
            commands::rename_db_backup,
            commands::delete_db_backup,
            commands::parse_deeplink,
            commands::import_from_deeplink_unified,
            commands::peek_pending_deeplink,
            commands::ack_pending_deeplink,
            commands::list_sessions,
            commands::get_session_messages,
            commands::stream_session_messages,
            commands::get_session_block_content,
            commands::get_session_image,
            commands::reveal_session_path,
            commands::export_session_markdown,
            commands::delete_session,
            commands::delete_sessions,
            commands::launch_session_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running cc-switch-desk");
}
