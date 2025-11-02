use baker::{
    cli::{parse_cli, get_log_level_from_verbose, Commands, run, Args, UpdateArgs},
    error::default_error_handler,
};

fn main() {
    let cli = parse_cli();
    // Determine verbosity from respective command args
    let (verbose, dispatch_result) = match &cli.command {
        Commands::Copy(args) => {
            let lvl = get_log_level_from_verbose(args.verbose);
            env_logger::Builder::new().filter_level(lvl).init();
            (args.verbose, run(args.clone()))
        }
        Commands::Update(upd) => {
            let lvl = get_log_level_from_verbose(upd.verbose);
            env_logger::Builder::new().filter_level(lvl).init();
            // Build Args equivalent for processing but mark update mode through runner modifications
            // We'll pass through a synthetic Args for reuse and signal update via metadata presence.
            let synthetic = Args {
                template: upd.template.clone().unwrap_or_else(|| "".into()),
                output_dir: upd.output_dir.clone(),
                force: false, // update never forces full overwrite of existing root
                verbose: upd.verbose,
                answers: upd.answers.clone(),
                skip_confirms: upd.skip_confirms.clone(),
                non_interactive: upd.non_interactive,
                dry_run: upd.dry_run,
            };
            (upd.verbose, baker::cli::runner::run_update(synthetic))
        }
    };

    if let Err(err) = dispatch_result {
        default_error_handler(err);
    }
}
