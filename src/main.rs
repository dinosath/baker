use baker::{
    cli::{get_args, get_log_level_from_verbose, run, run_update, Commands},
    error::default_error_handler,
};

fn main() {
    let args = get_args();
    let log_level = get_log_level_from_verbose(args.verbose);
    env_logger::Builder::new().filter_level(log_level).init();

    let result = match args.command {
        Commands::Generate(generate_args) => run(generate_args),
        Commands::Update(update_args) => run_update(update_args),
    };

    if let Err(err) = result {
        default_error_handler(err);
    }
}
