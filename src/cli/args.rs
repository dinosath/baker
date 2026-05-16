use crate::conflict::ConflictStyle;
use crate::constants::{exit_codes, verbosity};
use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand, ValueEnum};
use log::LevelFilter;
use std::fmt::Display;
use std::path::PathBuf;

const HELP_TEMPLATE: &str = r#"{about-section}
{usage-heading} {usage}

{all-args}
{after-help}
"#;

/// Skip confirmation prompts for specific stages.
#[derive(Debug, Clone, ValueEnum, Copy, PartialEq)]
#[value(rename_all = "lowercase")]
pub enum SkipConfirm {
    /// Skip every confirmation prompt.
    All,
    /// Skip file overwrite confirmations.
    Overwrite,
    /// Skip hook execution confirmations.
    Hooks,
}

impl Display for SkipConfirm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SkipConfirm::All => "all",
            SkipConfirm::Overwrite => "overwrite",
            SkipConfirm::Hooks => "hooks",
        };
        write!(f, "{s}")
    }
}

/// Arguments for the `generate` subcommand.
#[derive(Parser, Debug)]
pub struct GenerateArgs {
    /// Template directory or Git repository.
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Destination directory for generated files.
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Force overwrite of an existing output directory.
    #[arg(short, long)]
    pub force: bool,

    /// Predefined answers as JSON string or `-` to read from stdin.
    #[arg(short, long)]
    pub answers: Option<String>,

    /// Path to a JSON file containing predefined answers.
    #[arg(long = "answers-file", value_name = "FILE")]
    pub answers_file: Option<PathBuf>,

    /// Confirmation prompts to skip (comma-separated).
    #[arg(long = "skip-confirms", value_delimiter = ',')]
    #[arg(value_enum)]
    pub skip_confirms: Vec<SkipConfirm>,

    /// Disable interactive prompts when answers are provided.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Preview actions without touching the filesystem.
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Override the name of the generated-metadata file (default: .baker-generated.yaml).
    #[arg(long = "generated-file", value_name = "FILE")]
    pub generated_file: Option<String>,

    /// Override the conflict-marker style used during `baker update`.
    #[arg(long = "conflict-style", value_enum)]
    pub conflict_style: Option<ConflictStyle>,
}

/// Arguments for the `update` subcommand.
#[derive(Parser, Debug)]
pub struct UpdateArgs {
    /// Name of the generated-metadata file to read (default: .baker-generated.yaml).
    #[arg(long = "generated-file", value_name = "FILE")]
    pub generated_file: Option<String>,

    /// Extra answers as JSON string or `-` to read from stdin (merged on top of saved answers).
    #[arg(short, long)]
    pub answers: Option<String>,

    /// Path to a JSON file with extra answers (merged on top of saved answers).
    #[arg(long = "answers-file", value_name = "FILE")]
    pub answers_file: Option<PathBuf>,

    /// Override the conflict-marker style.
    #[arg(long = "conflict-style", value_enum)]
    pub conflict_style: Option<ConflictStyle>,

    /// Preview actions without touching the filesystem.
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Confirmation prompts to skip (comma-separated).
    #[arg(long = "skip-confirms", value_delimiter = ',')]
    #[arg(value_enum)]
    pub skip_confirms: Vec<SkipConfirm>,

    /// Disable interactive prompts when answers are provided.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

/// Baker subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a new project from a template.
    Generate(GenerateArgs),
    /// Update an existing generated project when the template changes.
    Update(UpdateArgs),
}

/// Top-level CLI arguments for Baker.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(help_template = HELP_TEMPLATE)]
pub struct Args {
    /// Increase logging verbosity (`-v`, `-vv`, `-vvv`).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

/// Parse command line arguments with custom handling for missing required inputs.
pub fn get_args() -> Args {
    Args::try_parse().unwrap_or_else(|e| {
        if e.kind() == ErrorKind::MissingRequiredArgument
            || e.kind() == ErrorKind::MissingSubcommand
        {
            let mut command = Args::command().help_template(HELP_TEMPLATE);
            if let Err(print_err) = command.print_help() {
                eprintln!("Failed to display help information: {print_err}");
            } else {
                println!();
            }
            std::process::exit(exit_codes::FAILURE);
        } else {
            e.exit();
        }
    })
}

/// Map `-v` counts to the appropriate log level.
pub fn get_log_level_from_verbose(verbose_count: u8) -> LevelFilter {
    match verbose_count {
        verbosity::OFF => LevelFilter::Error,
        verbosity::INFO => LevelFilter::Info,
        verbosity::DEBUG => LevelFilter::Debug,
        verbosity::TRACE.. => LevelFilter::Trace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_verbose_flags_to_log_filters() {
        use crate::constants::verbosity;
        assert_eq!(get_log_level_from_verbose(verbosity::OFF), LevelFilter::Error);
        assert_eq!(get_log_level_from_verbose(verbosity::INFO), LevelFilter::Info);
        assert_eq!(get_log_level_from_verbose(verbosity::DEBUG), LevelFilter::Debug);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE), LevelFilter::Trace);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE + 1), LevelFilter::Trace);
    }

    #[test]
    fn parses_minimal_generate_args() {
        use clap::Parser;
        let args = Args::parse_from(["baker", "generate", "template_dir", "output_dir"]);
        match args.command {
            Commands::Generate(g) => {
                assert_eq!(g.template, "template_dir");
                assert_eq!(g.output_dir, PathBuf::from("output_dir"));
            }
            _ => panic!("expected Generate"),
        }
    }

    #[test]
    fn parses_generate_with_force() {
        use clap::Parser;
        let args = Args::parse_from([
            "baker",
            "generate",
            "template_dir",
            "output_dir",
            "--force",
        ]);
        match args.command {
            Commands::Generate(g) => assert!(g.force),
            _ => panic!("expected Generate"),
        }
    }

    #[test]
    fn parses_update_subcommand() {
        use clap::Parser;
        let args = Args::parse_from(["baker", "update", "--dry-run"]);
        match args.command {
            Commands::Update(u) => assert!(u.dry_run),
            _ => panic!("expected Update"),
        }
    }

    #[test]
    fn parses_update_with_generated_file_override() {
        use clap::Parser;
        let args =
            Args::parse_from(["baker", "update", "--generated-file", "custom.yaml"]);
        match args.command {
            Commands::Update(u) => {
                assert_eq!(u.generated_file, Some("custom.yaml".to_string()))
            }
            _ => panic!("expected Update"),
        }
    }

    #[test]
    fn display_skip_confirm_variants() {
        assert_eq!(SkipConfirm::All.to_string(), "all");
        assert_eq!(SkipConfirm::Overwrite.to_string(), "overwrite");
        assert_eq!(SkipConfirm::Hooks.to_string(), "hooks");
    }

    #[test]
    fn parses_full_generate_flags() {
        use clap::Parser;
        let args = Args::parse_from([
            "baker",
            "generate",
            "template_dir",
            "output_dir",
            "--force",
            "-vvv",
            "--answers",
            "{\"name\":\"John\"}",
            "--skip-confirms",
            "all,overwrite",
            "--non-interactive",
            "--dry-run",
        ]);
        assert_eq!(args.verbose, 3);
        match args.command {
            Commands::Generate(g) => {
                assert!(g.force);
                assert_eq!(g.answers, Some("{\"name\":\"John\"}".to_string()));
                assert!(g.skip_confirms.contains(&SkipConfirm::All));
                assert!(g.non_interactive);
                assert!(g.dry_run);
            }
            _ => panic!("expected Generate"),
        }
    }

    #[test]
    fn parses_answers_file_argument() {
        use clap::Parser;
        let args = Args::parse_from([
            "baker",
            "generate",
            "template_dir",
            "output_dir",
            "--answers-file",
            "/path/to/answers.json",
        ]);
        match args.command {
            Commands::Generate(g) => {
                assert_eq!(g.answers_file, Some(PathBuf::from("/path/to/answers.json")));
            }
            _ => panic!("expected Generate"),
        }
    }
}
