//! Main entry point for the ned terminal modal text editor.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use std::process;

use ned::cli::cli_args;

/// RAII guard to clean up any temporary configuration file created for CLI session overrides.
struct temp_file_guard(Option<PathBuf>);

impl Drop for temp_file_guard {
    fn drop(&mut self) {
        if let Some(ref path) = self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let parsed = match cli_args::parse(args) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}\n");
            eprintln!("{}", cli_args::help_text());
            process::exit(2);
        }
    };

    if parsed.help {
        print!("{}", cli_args::help_text());
        return;
    }

    if parsed.version {
        println!("{}", cli_args::version_text());
        return;
    }

    if parsed.health {
        if let Err(e) = parsed.run_health() {
            eprintln!("ned health error: {e}");
            process::exit(1);
        }
        return;
    }

    if parsed.fetch_themes {
        if let Err(e) = parsed.run_fetch_themes() {
            eprintln!("ned fetch-themes error: {e}");
            process::exit(3);
        }
        return;
    }

    if parsed.fetch_grammars {
        if let Err(e) = parsed.run_fetch_grammars() {
            eprintln!("ned fetch-grammars error: {e}");
            process::exit(3);
        }
        return;
    }

    // Export explicit runtime path if specified on CLI
    if let Some(ref rt_path) = parsed.runtime {
        env::set_var("NED_RUNTIME", rt_path);
    }

    // Resolve effective configuration path
    let base_config_path: Option<PathBuf> = if parsed.no_config {
        // Pointing to empty virtual device skips loading user global and workspace configs
        if cfg!(windows) {
            Some(PathBuf::from("NUL"))
        } else {
            Some(PathBuf::from("/dev/null"))
        }
    } else {
        parsed.config.clone()
    };

    let mut _temp_guard = temp_file_guard(None);
    let effective_config_path = if let Some(ref theme_name) = parsed.theme {
        let mut content = String::new();
        if let Some(ref existing) = base_config_path {
            if let Ok(existing_content) = fs::read_to_string(existing) {
                content = existing_content;
            }
        }
        let _ = write!(content, "\n[editor]\ntheme = \"{theme_name}\"\n");
        let temp_path = env::temp_dir().join(format!("ned_cfg_override_{}.toml", process::id()));
        if fs::write(&temp_path, content).is_ok() {
            _temp_guard.0 = Some(temp_path.clone());
            Some(temp_path)
        } else {
            base_config_path
        }
    } else {
        base_config_path
    };

    if let Err(e) = ned::run_with_config(&parsed.files, effective_config_path.as_deref()) {
        eprintln!("ned error: {e}");
        process::exit(1);
    }
}
