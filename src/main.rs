use baker::{
    cli::{get_args, run},
    error::default_error_handler,
};

fn main() {
    let args = get_args();
    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Off
        })
        .init();

    if let Err(err) = run(args) {
        default_error_handler(err);
    }
}
