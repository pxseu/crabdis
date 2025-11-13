use log::{Level, LevelFilter};

pub fn init(verbose: bool) {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let level = record.level();

            match level {
                Level::Debug => out.finish(format_args!(
                    "{} [{}]: {}",
                    Level::Debug.to_string().to_lowercase(),
                    record.target(),
                    message
                )),

                level => out.finish(format_args!(
                    "{}: {}",
                    level.to_string().to_lowercase(),
                    message
                )),
            }
        })
        .level(if verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .chain(
            fern::Dispatch::new()
                .filter(|metadata| !matches!(metadata.level(), Level::Error | Level::Warn))
                .chain(std::io::stdout()),
        )
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Error)
                .level(log::LevelFilter::Warn)
                .chain(std::io::stderr()),
        )
        .apply()
        .ok();
}
