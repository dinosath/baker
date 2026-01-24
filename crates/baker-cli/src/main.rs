use baker_cli::{
    get_cli, get_log_level_from_verbose, run, Args, Commands, TemplateStore,
};
use baker_core::error::default_error_handler;

fn main() {
    let cli = get_cli();
    let log_level = get_log_level_from_verbose(cli.verbose);
    env_logger::Builder::new().filter_level(log_level).init();

    let result = match cli.command {
        Some(Commands::Generate(args)) => {
            let mut gen_args: Args = args.into();
            gen_args.verbose = cli.verbose;
            run(gen_args)
        }
        Some(Commands::Install(args)) => handle_install(args),
        Some(Commands::List) => handle_list(),
        Some(Commands::Remove(args)) => handle_remove(args.name),
        Some(Commands::Info(args)) => handle_info(args.name),
        #[cfg(feature = "mcp")]
        Some(Commands::Mcp) => handle_mcp(),
        None => {
            // Legacy mode: positional arguments
            if let (Some(template), Some(output_dir)) = (cli.template, cli.output_dir) {
                let args = Args {
                    template,
                    output_dir,
                    force: cli.force,
                    verbose: cli.verbose,
                    answers: cli.answers,
                    answers_file: cli.answers_file,
                    skip_confirms: cli.skip_confirms,
                    non_interactive: cli.non_interactive,
                    dry_run: cli.dry_run,
                };
                run(args)
            } else {
                // Show help if no command or arguments provided
                use clap::CommandFactory;
                let mut cmd = baker_cli::Cli::command();
                cmd.print_help().ok();
                println!();
                std::process::exit(0);
            }
        }
    };

    if let Err(err) = result {
        default_error_handler(err);
    }
}

fn handle_install(args: baker_cli::InstallArgs) -> baker_core::error::Result<()> {
    let store = TemplateStore::new()?;

    // Derive name from source if not provided
    let name = args.name.unwrap_or_else(|| derive_template_name(&args.source));

    store.install(&args.source, &name, args.description, args.force)?;
    Ok(())
}

fn handle_list() -> baker_core::error::Result<()> {
    let store = TemplateStore::new()?;
    let templates = store.list()?;

    if templates.is_empty() {
        println!("No templates installed.");
        println!();
        println!("Install a template with:");
        println!("  baker install <git-url-or-path> --name <name>");
        return Ok(());
    }

    println!("Installed templates:");
    println!();
    for t in templates {
        println!("  {} ", t.name);
        if let Some(desc) = &t.description {
            println!("    Description: {}", desc);
        }
        println!("    Source: {}", t.source);
        println!("    Installed: {}", t.installed_at);
        println!();
    }

    Ok(())
}

fn handle_remove(name: String) -> baker_core::error::Result<()> {
    let store = TemplateStore::new()?;
    store.remove(&name)?;
    Ok(())
}

fn handle_info(name: String) -> baker_core::error::Result<()> {
    let store = TemplateStore::new()?;
    let metadata = store.get_metadata(&name)?;

    println!("Template: {}", metadata.name);
    println!("Source: {}", metadata.source);
    println!("Installed: {}", metadata.installed_at);
    if let Some(desc) = metadata.description {
        println!("Description: {}", desc);
    }

    // Show archive size
    let archive_path = store.store_dir().join(format!("{}.tar.gz", name));
    if let Ok(meta) = std::fs::metadata(&archive_path) {
        let size = meta.len();
        let size_str = if size >= 1024 * 1024 {
            format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
        } else if size >= 1024 {
            format!("{:.2} KB", size as f64 / 1024.0)
        } else {
            format!("{} bytes", size)
        };
        println!("Archive size: {}", size_str);
    }

    Ok(())
}

/// Derives a template name from a source URL or path.
fn derive_template_name(source: &str) -> String {
    // Try to extract repo name from git URL
    if source.contains("github.com")
        || source.contains("gitlab.com")
        || source.ends_with(".git")
    {
        // Handle URLs like https://github.com/user/repo.git or git@github.com:user/repo.git
        let name = source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .or_else(|| source.rsplit(':').next())
            .unwrap_or(source);
        return name.to_string();
    }

    // For local paths, use the directory name
    std::path::Path::new(source)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("template")
        .to_string()
}

#[cfg(feature = "mcp")]
fn handle_mcp() -> baker_core::error::Result<()> {
    use baker_core::error::Error;

    // Build a tokio runtime for the MCP server
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        Error::Other(anyhow::anyhow!("Failed to create tokio runtime: {e}"))
    })?;

    rt.block_on(async {
        baker_cli::run_mcp_server()
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("MCP server error: {e}")))
    })
}
