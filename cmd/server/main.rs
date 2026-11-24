use clap::{Parser, Subcommand};
use site_core::config::Config;
use site_core::live_lists::start::{StartInputs, serve_tcp, start};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "folio")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server (default)
    Serve,
}

/// Reads configuration and calls the start function (R-5); the startup
/// sequence itself lives in `site_core::live_lists::start`. The environment
/// is read after `Config::from_env`, which loads `.env`.
async fn run_server() {
    let config = Config::from_env();
    let cors_origin = std::env::var("CORS_ORIGIN").ok();
    // The raw OS value: a value that is not Unicode must reach the parser as
    // such and never be read as unset (R-18).
    let switch = std::env::var_os("LIVE_LISTS_DISABLED");
    let port = config.port;
    let inputs = StartInputs::production(config, cors_origin, switch);
    match start(inputs, move |app| serve_tcp(port, app)).await {
        Ok(()) => {}
        Err(e) => {
            e.log();
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing — respects RUST_LOG env var, defaults to info
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            run_server().await;
        }
    }
}

/// R-29 (#4853): the release command and the `check-projects` subcommand are
/// retired. Absence is asserted here because `Cli` is private to the bin crate;
/// these tests name the removed path on purpose (R-29's grep exempts an inline
/// `#[cfg(test)]` module that does).
#[cfg(test)]
mod retired_release_command_tests {
    use super::*;

    const FLY_TOML: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fly.toml");

    /// Whether any non-comment line of a `fly.toml` text sets `release_command`.
    fn sets_release_command(text: &str) -> bool {
        text.lines().any(|l| {
            let l = l.trim_start();
            !l.starts_with('#') && l.starts_with("release_command")
        })
    }

    #[test]
    fn r29_the_real_fly_toml_has_no_release_command() {
        let text = std::fs::read_to_string(FLY_TOML).expect("fly.toml at the project root");
        assert!(!sets_release_command(&text));
    }

    #[test]
    fn r29_control_a_fly_toml_with_a_release_command_is_detected() {
        let seeded = "[deploy]\n  release_command = \"./folio check-projects\"\n";
        assert!(sets_release_command(seeded));
    }

    #[test]
    fn r29_check_projects_is_not_a_subcommand() {
        assert!(Cli::try_parse_from(["folio", "check-projects"]).is_err());
    }

    #[test]
    fn r29_no_subcommand_parses_to_none_which_serves() {
        let cli = Cli::try_parse_from(["folio"]).expect("bare folio parses");
        assert!(cli.command.is_none());
        assert!(matches!(
            Cli::try_parse_from(["folio", "serve"])
                .expect("serve parses")
                .command,
            Some(Commands::Serve)
        ));
    }
}
