use log::info;
use std::path::PathBuf;

const HOOK_SCRIPT_CONTENT: &str = include_str!("../../scripts/moyuguard-hook.sh");

fn moyuguard_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".moyuguard")
}

fn hook_script_path() -> PathBuf {
    moyuguard_dir().join("moyuguard-hook.sh")
}

fn ensure_hook_script() -> Result<(), String> {
    let dir = moyuguard_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create ~/.moyuguard: {}", e))?;

    let path = hook_script_path();
    std::fs::write(&path, HOOK_SCRIPT_CONTENT)
        .map_err(|e| format!("Failed to write hook script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to chmod hook script: {}", e))?;
    }

    info!("Hook script installed at {:?}", path);
    Ok(())
}

pub fn install_claude_code() -> Result<String, String> {
    ensure_hook_script()?;

    let settings_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json");

    let mut settings: serde_json::Value = if settings_path.exists() {
        let backup = settings_path.with_extension("json.moyuguard.bak");
        // Only create backup on first install; subsequent installs must not
        // overwrite the original-config backup with the moyuguard-modified one.
        if !backup.exists() {
            std::fs::copy(&settings_path, &backup)
                .map_err(|e| format!("Failed to backup settings: {}", e))?;
        }

        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings.json: {}", e))?
    } else {
        std::fs::create_dir_all(settings_path.parent().unwrap())
            .map_err(|e| format!("Failed to create ~/.claude: {}", e))?;
        serde_json::json!({})
    };

    let hook_cmd = hook_script_path().to_string_lossy().to_string();

    let hooks = settings.as_object_mut()
        .ok_or("settings.json is not an object")?
        .entry("hooks")
        .or_insert(serde_json::json!({}));

    let hook_obj = hooks.as_object_mut()
        .ok_or("hooks is not an object")?;

    // PermissionRequest is the only blocking hook — Claude Code re-reads it
    // live, so it works even in already-running sessions.
    // All other events are fire-and-forget (async telemetry only), matching
    // CodeIsland's routing model.
    let blocking_events = vec![("PermissionRequest", 86400u32)];
    let fire_and_forget_events = vec![
        ("PreToolUse", 5u32),
        ("PostToolUse", 5),
        ("Notification", 5),
        ("Stop", 5),
        ("SessionStart", 5),
        ("SessionEnd", 5),
    ];

    for (event, timeout) in &blocking_events {
        let entry = make_hook_entry(&hook_cmd, *timeout);
        merge_hook_entry(hook_obj, event, entry);
    }

    for (event, timeout) in &fire_and_forget_events {
        let entry = make_hook_entry(&hook_cmd, *timeout);
        merge_hook_entry(hook_obj, event, entry);
    }

    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&settings_path, output)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    info!("Claude Code hooks installed at {:?}", settings_path);
    Ok(format!("Claude Code hooks installed. {} events configured.", blocking_events.len() + fire_and_forget_events.len()))
}

pub fn install_codex() -> Result<String, String> {
    ensure_hook_script()?;

    let codex_home = std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".codex"));

    std::fs::create_dir_all(&codex_home)
        .map_err(|e| format!("Failed to create codex home: {}", e))?;

    let hooks_path = codex_home.join("hooks.json");
    let hook_cmd = hook_script_path().to_string_lossy().to_string();

    let mut hooks: serde_json::Value = if hooks_path.exists() {
        let backup = hooks_path.with_extension("json.moyuguard.bak");
        if !backup.exists() {
            std::fs::copy(&hooks_path, &backup).ok();
        }

        let content = std::fs::read_to_string(&hooks_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let hook_obj = hooks.as_object_mut()
        .ok_or("hooks.json is not an object")?;

    let codex_events: &[(&str, u32)] = &[
        ("PermissionRequest", 86400),
        ("PreToolUse", 5),
        ("PostToolUse", 5),
        ("Notification", 5),
        ("Stop", 5),
        ("SessionStart", 5),
        ("SessionEnd", 5),
    ];
    for (event, timeout) in codex_events {
        let entry = make_codex_hook_entry(&hook_cmd, *timeout);
        merge_hook_entry(hook_obj, event, entry);
    }

    let output = serde_json::to_string_pretty(&hooks)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&hooks_path, output)
        .map_err(|e| format!("Failed to write hooks.json: {}", e))?;

    let config_path = codex_home.join("config.toml");
    let config_content = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if !content.contains("hooks = true") {
            if content.contains("[features]") {
                content.replace("[features]", "[features]\nhooks = true")
            } else {
                format!("{}\n\n[features]\nhooks = true\n", content)
            }
        } else {
            content
        }
    } else {
        "[features]\nhooks = true\n".to_string()
    };
    std::fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write config.toml: {}", e))?;

    info!("Codex hooks installed at {:?}", hooks_path);
    Ok("Codex hooks installed.".to_string())
}

pub fn uninstall_all() -> Result<String, String> {
    let mut results = Vec::new();

    // Claude Code: prefer restoring from backup so previously-installed hooks
    // (e.g. CodeIsland) come back intact. Fall back to key-removal if no backup.
    let settings_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json");
    let claude_backup = settings_path.with_extension("json.moyuguard.bak");

    if claude_backup.exists() {
        match std::fs::copy(&claude_backup, &settings_path) {
            Ok(_) => {
                let _ = std::fs::remove_file(&claude_backup);
                results.push("Claude Code: restored settings.json from backup".to_string());
            }
            Err(e) => results.push(format!("Claude Code: failed to restore backup - {}", e)),
        }
    } else if settings_path.exists() {
        match std::fs::read_to_string(&settings_path) {
            Ok(content) => {
                if let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
                        let events_to_remove: Vec<String> = hooks.keys()
                            .filter(|k| {
                                is_moyuguard_hook(hooks.get(*k).unwrap())
                            })
                            .cloned()
                            .collect();

                        for event in &events_to_remove {
                            hooks.remove(event);
                        }

                        if !events_to_remove.is_empty() {
                            let output = serde_json::to_string_pretty(&settings).unwrap_or_default();
                            std::fs::write(&settings_path, output).ok();
                            results.push(format!("Claude Code: removed {} hook events", events_to_remove.len()));
                        }
                    }
                }
            }
            Err(e) => results.push(format!("Claude Code: failed to read - {}", e)),
        }
    }

    // Codex: same backup-first strategy
    let codex_home = std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".codex"));
    let hooks_path = codex_home.join("hooks.json");
    let codex_backup = hooks_path.with_extension("json.moyuguard.bak");

    if codex_backup.exists() {
        match std::fs::copy(&codex_backup, &hooks_path) {
            Ok(_) => {
                let _ = std::fs::remove_file(&codex_backup);
                results.push("Codex: restored hooks.json from backup".to_string());
            }
            Err(e) => results.push(format!("Codex: failed to restore backup - {}", e)),
        }
    } else if hooks_path.exists() {
        match std::fs::read_to_string(&hooks_path) {
            Ok(content) => {
                if let Ok(mut hooks) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = hooks.as_object_mut() {
                        let to_remove: Vec<String> = obj.keys()
                            .filter(|k| is_moyuguard_hook(obj.get(*k).unwrap()))
                            .cloned()
                            .collect();

                        for event in &to_remove {
                            obj.remove(event);
                        }

                        if !to_remove.is_empty() {
                            let output = serde_json::to_string_pretty(&hooks).unwrap_or_default();
                            std::fs::write(&hooks_path, output).ok();
                            results.push(format!("Codex: removed {} hook events", to_remove.len()));
                        }
                    }
                }
            }
            Err(e) => results.push(format!("Codex: failed to read - {}", e)),
        }
    }

    if results.is_empty() {
        Ok("No hooks found to uninstall.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

pub fn get_hook_status() -> serde_json::Value {
    let claude_installed = check_claude_hooks();
    let codex_installed = check_codex_hooks();

    serde_json::json!({
        "claude_code": claude_installed,
        "codex": codex_installed,
    })
}

fn check_claude_hooks() -> bool {
    let path = dirs::home_dir().unwrap_or_default().join(".claude").join("settings.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(hooks) = settings.get("hooks") {
                return hooks.as_object()
                    .map(|h| h.values().any(|v| is_moyuguard_hook(v)))
                    .unwrap_or(false);
            }
        }
    }
    false
}

fn check_codex_hooks() -> bool {
    let codex_home = std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".codex"));
    let path = codex_home.join("hooks.json");

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(hooks) = serde_json::from_str::<serde_json::Value>(&content) {
            return hooks.as_object()
                .map(|h| h.values().any(|v| is_moyuguard_hook(v)))
                .unwrap_or(false);
        }
    }
    false
}

fn is_moyuguard_hook(value: &serde_json::Value) -> bool {
    let s = serde_json::to_string(value).unwrap_or_default();
    s.contains("moyuguard-hook.sh")
}

fn make_hook_entry(command: &str, timeout: u32) -> serde_json::Value {
    serde_json::json!([
        {
            "matcher": "",
            "hooks": [
                {
                    "type": "command",
                    "command": command,
                    "timeout": timeout
                }
            ]
        }
    ])
}

fn make_codex_hook_entry(command: &str, timeout: u32) -> serde_json::Value {
    serde_json::json!([
        {
            "hooks": [
                {
                    "type": "command",
                    "command": command,
                    "timeout": timeout
                }
            ]
        }
    ])
}

/// Append MoyuGuard hook entries to an existing event key without clobbering
/// hooks from other tools (e.g. CodeIsland).
///
/// Strategy:
///   1. Ensure the key exists as an array.
///   2. Remove any previous MoyuGuard entries (idempotent reinstall).
///   3. Append the new entries from `new_entries` (which is itself an array).
fn merge_hook_entry(
    hook_obj: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    new_entries: serde_json::Value,
) {
    let slot = hook_obj
        .entry(event.to_string())
        .or_insert_with(|| serde_json::json!([]));

    // If the existing value is somehow not an array, replace it wholesale.
    if !slot.is_array() {
        *slot = new_entries;
        return;
    }

    let arr = slot.as_array_mut().unwrap();

    // Remove old MoyuGuard entries so reinstall is idempotent.
    arr.retain(|h| !is_moyuguard_hook(h));

    // Append new entries.
    if let Some(new_arr) = new_entries.as_array() {
        arr.extend(new_arr.iter().cloned());
    }
}
