mod commands;
mod dependencies;
mod terraform;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            // Focus the existing window if app is already running
        }))
        .setup(|app| {
            // Extract templates to app data directory on first run
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(e) = commands::setup_templates(&app_handle) {
                    eprintln!("Failed to setup templates: {}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::install_terraform,
            commands::validate_databricks_credentials,
            commands::get_templates,
            commands::get_template_variables,
            commands::save_configuration,
            commands::run_terraform_command,
            commands::get_deployment_status,
            commands::cancel_deployment,
            commands::rollback_deployment,
            commands::get_cloud_credentials,
            commands::get_aws_profiles,
            commands::get_aws_identity,
            commands::aws_sso_login,
            commands::get_azure_account,
            commands::get_azure_subscriptions,
            commands::get_azure_resource_groups,
            commands::azure_login,
            commands::set_azure_subscription,
            commands::clear_templates_cache,
            commands::get_deployments_folder,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
