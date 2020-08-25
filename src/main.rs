

#[derive(Debug)]
struct CliError { 
    pub message: String
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "{}", self.message) 
    }
}

fn main() -> Result<(), CliError> {
    // Define the acceptable user input behavior
    let matches = clap::App::new("VR Actuators")
        .version("v0.1")
        .author("Jacob Trueb <jtrueb@northwestern.edu")
        .about("CLI entrypoint to ZMQ")
        .arg(clap::Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(clap::App::new("req")
            .about("Starts tries to connect a req client to something")
            .arg(clap::Arg::with_name("routine")
                .short("r")
                .long("routine")
                .value_name("ROUTINE")
                .default_value("default")
                .help("The routine to run once connected")
                .takes_value(true))
            .arg(clap::Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .value_name("endpoint")
                .help("Bind like tcp://*:0 or Connect like tcp://0.0.0.0:5555")
                .takes_value(true)
                .required(true)))
        .get_matches();

    // Configure the logger before heading off to the rest of the functionality
    simple_logger::init().unwrap(); 
    let level_filter = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Trace,
    };
    log::set_max_level(level_filter);
    log::debug!("Found level_filter: {}", level_filter);

    if let Some(matches) = matches.subcommand_matches("req") {
        log::info!("Preparing to connect REQ socket");
        log::trace!("CLI Params: {:#?}", matches);

        let endpoint = matches.value_of("endpoint").unwrap();
        
        match matches.value_of("routine") {
            Some("default") => {
                log::info!("Running default routine for req after connect to {}", endpoint);
            },
            _ => {
                log::error!("Unknown routine for socket of type req");
            }
        }
    }

    Ok(())
}
