use crate::config::Config;

use crate::utils::version;
use clap::{value_parser, Arg, Command, Error};
use std::env;

pub fn update_config(config: Config) -> Result<Config, Error> {
    let matches = Command::new(clap::crate_name!())
        .author(clap::crate_authors!())
        .version(version())
        .about(clap::crate_description!())
        .arg(
            Arg::new("tick_rate")
                .value_parser(value_parser!(f64))
                .short('t')
                .long("tick_rate")
                .value_name("FLOAT")
                .help("Tick rate, i.e. number of ticks per second")
                .default_value(config.tick_rate.to_string()),
        )
        .arg(
            Arg::new("frame_rate")
                .value_parser(value_parser!(f64))
                .short('f')
                .long("frame_rate")
                .value_name("FLOAT")
                .help("Frame rate, i.e. number of frames per second")
                .default_value(config.frame_rate.to_string()),
        )
        .try_get_matches()?;

    let _tick_rate = *matches.get_one::<f64>("tick_rate").unwrap();
    let _frame_rate = *matches.get_one::<f64>("frame_rate").unwrap();

    Ok(config)
}
