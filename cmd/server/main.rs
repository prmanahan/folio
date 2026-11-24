use clap::{Parser, Subcommand};
use site_core::config::{Config, DEFAULT_STATIC_DIR};
use site_core::live_lists::start::{StartError, StartInputs, serve_tcp, start};
use site_core::projects_list;
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
    /// Validate `$STATIC_DIR/projects.json` and its freshness, then exit
    /// (Fly release command). Reads no secrets and opens no database.
    CheckProjects,
}

/// R-18. Deliberately independent of `Config`: it must run on Fly's release
/// machine, which has no volume, and must not require or read the app's
/// secrets. Returns whether the list passed.
fn check_projects() -> bool {
    let static_dir = std::env::var_os("STATIC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_STATIC_DIR));
    let path = static_dir.join("projects.json");

    // `%e` is safe for untrusted text: every `Display` arm that interpolates
    // text from the file does so through `{:?}`, which escapes control
    // characters, independent of the order the checks ran in. Pinned per arm
    // in `libs/site-core/tests/test_projects_list_validator.rs`.
    let list = match projects_list::load(&path) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "check-projects failed");
            return false;
        }
    };
    let age = match projects_list::check_fetched_at_age_now(list.fetched_at()) {
        Ok(age) => age,
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "check-projects failed");
            return false;
        }
    };
    let counts: Vec<String> = list.counts().map(|(o, n)| format!("{o}={n}")).collect();
    tracing::info!(
        path = %path.display(),
        fetched_at = list.fetched_at(),
        age_secs = age.0.whole_seconds(),
        bound = format!(
            "{} minutes behind, {} minutes ahead",
            projects_list::MAX_AGE_BEHIND.whole_minutes(),
            projects_list::MAX_AGE_AHEAD.whole_minutes()
        ),
        counts = counts.join(" "),
        "check-projects passed"
    );
    true
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
        // Already logged, naming the path.
        Err(StartError::ProjectsListRefused) => std::process::exit(1),
        Err(e) => panic!("server start failed: {e:?}"),
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
        Commands::CheckProjects => {
            if !check_projects() {
                std::process::exit(1);
            }
        }
    }
}

/// R-19 (#4726): `fly.toml` carries the release command, and the command
/// parses to R-18's subcommand. Lives here because `Cli` is private to the
/// bin crate. Written for plan T6 (anchor D8); the real `fly.toml` now carries
/// the `[deploy]` table, and these tests hold it to the command.
#[cfg(test)]
mod release_command_tests {
    use super::*;
    use toml_edit::DocumentMut;

    const FLY_TOML: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fly.toml");
    const RELEASE_COMMAND: &str = "./folio check-projects";

    /// What R-19 asserts about the text of a `fly.toml`.
    fn check_fly_toml(text: &str) -> Result<(), String> {
        let doc: DocumentMut = text.parse().map_err(|e| format!("not TOML: {e}"))?;
        let deploy = doc.get("deploy").ok_or("no [deploy] table")?;
        let value = deploy
            .get("release_command")
            .ok_or("no deploy.release_command")?
            .as_str()
            .ok_or("deploy.release_command is not a string")?;
        if value != RELEASE_COMMAND {
            return Err(format!(
                "deploy.release_command is {value:?}, not {RELEASE_COMMAND:?}"
            ));
        }
        let mut tokens = value.split_whitespace();
        if tokens.next() != Some("./folio") {
            return Err("the first token is not ./folio".to_string());
        }
        let rest: Vec<&str> = tokens.collect();
        if !yields_check_projects(&rest) {
            return Err(format!(
                "`folio {}` is not the check-projects subcommand",
                rest.join(" ")
            ));
        }
        Ok(())
    }

    /// `argv[0]` comes first, so a subcommand is never read as the program.
    fn yields_check_projects(tokens_after_folio: &[&str]) -> bool {
        let argv = std::iter::once("folio").chain(tokens_after_folio.iter().copied());
        matches!(
            Cli::try_parse_from(argv),
            Ok(Cli {
                command: Some(Commands::CheckProjects)
            })
        )
    }

    fn real_fly_toml() -> String {
        std::fs::read_to_string(FLY_TOML).expect("fly.toml at the project root")
    }

    /// A copy of the real file with a `[deploy]` table added whose
    /// `release_command` is `value`. Seeded copies start from the real file
    /// minus any `[deploy]` table, so they stay valid once T8 adds one.
    fn with_deploy(value: &str) -> String {
        format!(
            "{}\n[deploy]\n  release_command = \"{value}\"\n",
            without_deploy_table(&real_fly_toml())
        )
    }

    /// The real file with its `[deploy]` table (header through the next
    /// table header or the end) removed.
    fn without_deploy_table(text: &str) -> String {
        let mut out = String::new();
        let mut skipping = false;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                skipping = t == "[deploy]";
            }
            if !skipping {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn r19_the_real_fly_toml_carries_the_release_command() {
        // Given the project's fly.toml
        // Then deploy.release_command is `./folio check-projects`, and it
        // parses to the check-projects subcommand
        if let Err(why) = check_fly_toml(&real_fly_toml()) {
            panic!("fly.toml fails R-19: {why}");
        }
    }

    #[test]
    fn r19_control_a_seeded_copy_with_the_right_table_and_key_passes() {
        assert_eq!(check_fly_toml(&with_deploy(RELEASE_COMMAND)), Ok(()));
    }

    #[test]
    fn r19_seeded_copy_without_a_deploy_table_fails() {
        assert!(check_fly_toml(&without_deploy_table(&real_fly_toml())).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_a_deploy_table_and_no_release_command_fails() {
        let text = format!(
            "{}\n[deploy]\n  strategy = \"rolling\"\n",
            without_deploy_table(&real_fly_toml())
        );
        assert!(check_fly_toml(&text).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_release_command_true_fails() {
        assert!(check_fly_toml(&with_deploy("true")).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_release_command_bare_folio_fails() {
        assert!(check_fly_toml(&with_deploy("./folio")).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_release_command_folio_serve_fails() {
        assert!(check_fly_toml(&with_deploy("./folio serve")).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_a_misspelled_subcommand_fails() {
        // A subcommand that does not exist: the parse, not only the string
        assert!(check_fly_toml(&with_deploy("./folio check-project")).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_the_release_command_under_another_table_fails() {
        let text = format!(
            "{}\n[other]\n  release_command = \"{RELEASE_COMMAND}\"\n",
            without_deploy_table(&real_fly_toml())
        );
        assert!(check_fly_toml(&text).is_err());
    }

    #[test]
    fn r19_seeded_copy_with_the_release_command_in_a_nested_deploy_table_fails() {
        let text = format!(
            "{}\n[deploy.other]\n  release_command = \"{RELEASE_COMMAND}\"\n",
            without_deploy_table(&real_fly_toml())
        );
        assert!(check_fly_toml(&text).is_err());
    }

    #[test]
    fn r19_tokens_after_folio_yield_check_projects() {
        assert!(yields_check_projects(&["check-projects"]));
    }

    #[test]
    fn r19_no_subcommand_does_not_yield_check_projects() {
        // `./folio` alone parses to no subcommand, which maps to Serve
        assert!(!yields_check_projects(&[]));
    }

    #[test]
    fn r19_serve_does_not_yield_check_projects() {
        assert!(!yields_check_projects(&["serve"]));
    }

    #[test]
    fn r19_a_renamed_subcommand_does_not_parse() {
        assert!(Cli::try_parse_from(["folio", "check-project"]).is_err());
    }
}
