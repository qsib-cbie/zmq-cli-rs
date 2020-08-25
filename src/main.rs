use rand::Rng;

#[derive(Debug)]
enum CliError {
    ZmqError(zmq::Error),
    InvalidStringError(String),
}

impl std::convert::From<zmq::Error> for CliError {
    fn from(err: zmq::Error) -> Self {
        CliError::ZmqError(err)
    }
}

impl std::convert::From<std::vec::Vec<u8>> for CliError {
    fn from(invalid_string_data: std::vec::Vec<u8>) -> Self {
        CliError::InvalidStringError(hex(&invalid_string_data))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("")
}

struct RtDealer {}

impl RtDealer {
    pub fn router(endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting ROUTER for rtdealer");

        // Prepare the router socket to listen on the endpoint
        let ctx = zmq::Context::new();
        let broker = ctx.socket(zmq::ROUTER)?;
        broker.bind(endpoint)?;

        // Start communicating on the endpoint
        let start_time = std::time::Instant::now();
        let mut workers_fired = 0;
        let worker_pool_size = 4;
        let allowed_duration = std::time::Duration::new(30, 0);
        loop {
            let identity = broker.recv_bytes(0)?;
            broker.send(&identity, zmq::SNDMORE)?;

            broker.recv_bytes(0)?; // Receive envelope
            broker.recv_bytes(0)?; // Receive response

            broker.send("", zmq::SNDMORE)?; // Reply

            if start_time.elapsed() < allowed_duration {
                broker.send("Work harder", 0)?;
            } else {
                broker.send("Fired!", 0)?;
                workers_fired += 1;
                if workers_fired >= worker_pool_size {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn dealer(endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting DEALER for rtdealer");

        // Prepare the delear socket to connect to the endpoint
        let ctx = zmq::Context::new();
        let worker = ctx.socket(zmq::DEALER)?;

        let mut rng = rand::thread_rng();
        let identity: Vec<_> = (0..10).map(|_| rand::random::<u8>()).collect();
        worker.set_identity(&identity)?;
        worker.connect(endpoint)?;

        let mut total = 0;
        loop {
            // Tell the broker we are ready for work
            worker.send("", zmq::SNDMORE)?;
            worker.send("Hi boss!", 0)?;

            // Get workload from broker, until finished
            worker.recv_bytes(0)?;
            let workload = worker.recv_string(0)??;
            if workload == "Fired!" {
                log::error!("Worker {} completed {} tasks", hex(&identity), total);
                break;
            }
            total += 1;

            // Do some random work
            std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(1,500)));

            log::debug!("Finished work!");
        }


        Ok(())
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
        .subcommand(clap::App::new("start")
            .about("Starts a socket running some routine")
            .arg(clap::Arg::with_name("socket_type")
                .short("s")
                .long("socket-type")
                .value_name("SOCKET_TYPE")
                .help("The type of socket to create")
                .takes_value(true)
                .required(true))
            .arg(clap::Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .value_name("endpoint")
                .help("Bind like tcp://*:0 or Connect like tcp://0.0.0.0:5555")
                .takes_value(true)
                .required(true))
            .arg(clap::Arg::with_name("routine")
                .short("r")
                .long("routine")
                .value_name("ROUTINE")
                .help("The routine to run once connected")
                .takes_value(true)))
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

    if let Some(matches) = matches.subcommand_matches("start") {
        log::trace!("CLI Params: {:#?}", matches);

        let (socket_type, socket_type_name)  = match matches.value_of("socket_type") {
            Some("router") => {
                (zmq::ROUTER, "router")
            },
            Some("dealer") => {
                (zmq::DEALER, "dealer")
            },
            Some(unknown) => {
                log::error!("Unknown socket type: {}", unknown);
                panic!("Invalid socket type");
            },
            _ => {
                log::error!("Socket type must be specified");
                panic!("No socket type specified");
            }
        };

        let endpoint = matches.value_of("endpoint").unwrap();

        log::info!("Preparing {} socket for {}", socket_type_name, endpoint);
        
        match matches.value_of("routine") {
            Some("rtdealer") => {
                match socket_type {
                    zmq::ROUTER => {
                        return RtDealer::router(endpoint);
                    },
                    zmq::DEALER => {
                        return RtDealer::dealer(endpoint);
                    },
                    _ => {
                        let routine = "rtdealer";
                        log::error!("Invalid socket type {} for routine {}", socket_type_name, routine);
                        panic!("Cannot procede");
                    }
                }

            },
            _ => {
                log::error!("Unknown routine for socket of type req");
            }
        }
    }

    Ok(())
}
