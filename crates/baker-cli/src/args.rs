use baker_core::constants::{exit_codes, verbosity};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
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

impl From<SkipConfirm> for baker_core::types::SkipConfirm {
    fn from(value: SkipConfirm) -> Self {
        match value {
            SkipConfirm::All => baker_core::types::SkipConfirm::All,
            SkipConfirm::Overwrite => baker_core::types::SkipConfirm::Overwrite,
            SkipConfirm::Hooks => baker_core::types::SkipConfirm::Hooks,
        }
    }
}

/// Baker - A project scaffolding tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Increase logging verbosity (`-v`, `-vv`, `-vvv`).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,

    // Legacy positional arguments for backward compatibility
    /// Template directory, Git repository, or installed template name.
    #[arg(value_name = "TEMPLATE")]
    pub template: Option<String>,

    /// Destination directory for generated files.
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: Option<PathBuf>,

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
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a project from a template.
    #[command(visible_alias = "gen")]
    Generate(GenerateArgs),

    /// Install a template from a git repository or local path.
    Install(InstallArgs),

    /// List installed templates.
    #[command(visible_alias = "ls")]
    List,

    /// Remove an installed template.
    #[command(visible_alias = "rm")]
    Remove(RemoveArgs),

    /// Show information about an installed template.
    Info(InfoArgs),

    /// Start the MCP (Model Context Protocol) server.
    ///
    /// This starts Baker as an MCP server that exposes template listing
    /// and generation as tools for AI assistants.
    #[cfg(feature = "mcp")]
    Mcp,
}

/// Arguments for the generate command.
#[derive(Parser, Debug)]
pub struct GenerateArgs {
    /// Template directory, Git repository, or installed template name.
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
}

/// Arguments for the install command.
#[derive(Parser, Debug)]
pub struct InstallArgs {
    /// Git repository URL or local path to the template.
    #[arg(value_name = "SOURCE")]
    pub source: String,

    /// Name for the installed template (defaults to repository/directory name).
    #[arg(short, long)]
    pub name: Option<String>,

    /// Description for the template.
    #[arg(short, long)]
    pub description: Option<String>,

    /// Overwrite if template already exists.
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the remove command.
#[derive(Parser, Debug)]
pub struct RemoveArgs {
    /// Name of the template to remove.
    #[arg(value_name = "NAME")]
    pub name: String,
}

/// Arguments for the info command.
#[derive(Parser, Debug)]
pub struct InfoArgs {
    /// Name of the template to show info for.
    #[arg(value_name = "NAME")]
    pub name: String,
}

/// CLI arguments for Baker (legacy structure for backward compatibility).
#[derive(Debug)]
pub struct Args {
    /// Template directory or Git repository.
    pub template: String,

    /// Destination directory for generated files.
    pub output_dir: PathBuf,

    /// Force overwrite of an existing output directory.
    pub force: bool,

    /// Verbosity level.
    pub verbose: u8,

    /// Predefined answers as JSON string or `-` to read from stdin.
    pub answers: Option<String>,

    /// Path to a JSON file containing predefined answers.
    pub answers_file: Option<PathBuf>,

    /// Confirmation prompts to skip (comma-separated).
    pub skip_confirms: Vec<SkipConfirm>,

    /// Disable interactive prompts when answers are provided.
    pub non_interactive: bool,

    /// Preview actions without touching the filesystem.
    pub dry_run: bool,
}

/// Parse command line arguments.
pub fn get_cli() -> Cli {
    Cli::parse()
}

/// Parse command line arguments with custom handling for missing required inputs.
/// This is for backward compatibility with the legacy API.
pub fn get_args() -> Args {
    let cli = Cli::parse();

    // If using subcommand, this shouldn't be called
    if cli.command.is_some() {
        eprintln!("Error: get_args() called with subcommand. Use get_cli() instead.");
        std::process::exit(exit_codes::FAILURE);
    }

    // Check for required positional arguments
    let template = match cli.template {
        Some(t) => t,
        None => {
            let mut command = Cli::command().help_template(HELP_TEMPLATE);
            if let Err(print_err) = command.print_help() {
                eprintln!("Failed to display help information: {print_err}");
            } else {
                println!();
            }
            std::process::exit(exit_codes::FAILURE);
        }
    };

    let output_dir = match cli.output_dir {
        Some(o) => o,
        None => {
            let mut command = Cli::command().help_template(HELP_TEMPLATE);
            if let Err(print_err) = command.print_help() {
                eprintln!("Failed to display help information: {print_err}");
            } else {
                println!();
            }
            std::process::exit(exit_codes::FAILURE);
        }
    };

    Args {
        template,
        output_dir,
        force: cli.force,
        verbose: cli.verbose,
        answers: cli.answers,
        answers_file: cli.answers_file,
        skip_confirms: cli.skip_confirms,
        non_interactive: cli.non_interactive,
        dry_run: cli.dry_run,
    }
}

impl From<GenerateArgs> for Args {
    fn from(args: GenerateArgs) -> Self {
        Args {
            template: args.template,
            output_dir: args.output_dir,
            force: args.force,
            verbose: 0, // Will be set from Cli
            answers: args.answers,
            answers_file: args.answers_file,
            skip_confirms: args.skip_confirms,
            non_interactive: args.non_interactive,
            dry_run: args.dry_run,
        }
    }
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
        use baker_core::constants::verbosity;
        assert_eq!(get_log_level_from_verbose(verbosity::OFF), LevelFilter::Error);
        assert_eq!(get_log_level_from_verbose(verbosity::INFO), LevelFilter::Info);
        assert_eq!(get_log_level_from_verbose(verbosity::DEBUG), LevelFilter::Debug);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE), LevelFilter::Trace);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE + 1), LevelFilter::Trace);
    }

    #[test]
    fn parses_legacy_minimal_args() {
        use clap::Parser;
        let cli = Cli::parse_from(["baker", "template_dir", "output_dir", "--force"]);
        assert_eq!(cli.template, Some("template_dir".to_string()));
        assert_eq!(cli.output_dir, Some(PathBuf::from("output_dir")));
        assert!(cli.force);
    }

    #[test]
    fn display_skip_confirm_variants() {
        assert_eq!(SkipConfirm::All.to_string(), "all");
        assert_eq!(SkipConfirm::Overwrite.to_string(), "overwrite");
        assert_eq!(SkipConfirm::Hooks.to_string(), "hooks");
    }

    #[test]
    fn parses_generate_subcommand() {
        use clap::Parser;
        let cli = Cli::parse_from([
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
        assert_eq!(cli.verbose, 3);
        match cli.command {
            Some(Commands::Generate(args)) => {
                assert_eq!(args.template, "template_dir");
                assert_eq!(args.output_dir, PathBuf::from("output_dir"));
                assert!(args.force);
                assert_eq!(args.answers, Some("{\"name\":\"John\"}".to_string()));
                assert!(args.skip_confirms.contains(&SkipConfirm::All));
                assert!(args.skip_confirms.contains(&SkipConfirm::Overwrite));
                assert!(args.non_interactive);
                assert!(args.dry_run);
            }
            _ => panic!("Expected Generate command"),
        }
    }

    #[test]
    fn parses_install_subcommand() {
        use clap::Parser;
        let cli = Cli::parse_from([
            "baker",
            "install",
            "https://github.com/user/template",
            "--name",
            "my-template",
            "--description",
            "A cool template",
            "--force",
        ]);
        match cli.command {
            Some(Commands::Install(args)) => {
                assert_eq!(args.source, "https://github.com/user/template");
                assert_eq!(args.name, Some("my-template".to_string()));
                assert_eq!(args.description, Some("A cool template".to_string()));
                assert!(args.force);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn parses_list_subcommand() {
        use clap::Parser;
        let cli = Cli::parse_from(["baker", "list"]);
        assert!(matches!(cli.command, Some(Commands::List)));
    }

    #[test]
    fn parses_remove_subcommand() {
        use clap::Parser;
        let cli = Cli::parse_from(["baker", "remove", "my-template"]);
        match cli.command {
            Some(Commands::Remove(args)) => {
                assert_eq!(args.name, "my-template");
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn parses_answers_file_argument() {
        use clap::Parser;
        let cli = Cli::parse_from([
            "baker",
            "generate",
            "template_dir",
            "output_dir",
            "--answers-file",
            "/path/to/answers.json",
        ]);
        match cli.command {
            Some(Commands::Generate(args)) => {
                assert_eq!(args.template, "template_dir");
                assert_eq!(args.output_dir, PathBuf::from("output_dir"));
                assert_eq!(
                    args.answers_file,
                    Some(PathBuf::from("/path/to/answers.json"))
                );
            }
            _ => panic!("Expected Generate command"),
        }
    }
}
