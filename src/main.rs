use rand::Rng;
use std::io::prelude::*;

#[derive(Debug)]
enum CliError {
    StdError(Box<dyn std::error::Error>),
    IoError(std::io::Error),
    ZmqError(zmq::Error),
    InvalidStringError(String),
}

impl std::convert::From<Box<dyn std::error::Error>> for CliError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        CliError::StdError(err)
    }
}

impl std::convert::From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::IoError(err)
    }
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
                    log::info!("All workers are fired");
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
                log::info!("Worker {} completed {} tasks", hex(&identity), total);
                break;
            }
            total += 1;

            // Do some random work
            std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(1,50)));

            log::debug!("Finished work!");
        }


        Ok(())
    }

}

struct WuProxy { }

impl WuProxy {
    fn wuproxy(front_endpoint: &str, back_endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting proxy for {} and {}", front_endpoint, back_endpoint);

        let ctx = zmq::Context::new();
        let frontend = ctx.socket(zmq::XSUB)?;
        let backend = ctx.socket(zmq::XPUB)?;

        frontend.bind(front_endpoint)?;
        backend.bind(back_endpoint)?;

        log::info!("Bound and beginning proxy");

        zmq::proxy(&frontend, &backend)?;

        Ok(())
    }

    fn wuclient(endpoint: &str, filter: Option<&str>) -> Result<(), CliError> {
        log::info!("Starting client for {} with filter '{}'", endpoint, filter.unwrap_or(""));

        let ctx = zmq::Context::new();
        let subscriber = ctx.socket(zmq::SUB)?;
        subscriber.connect(endpoint)?;

        let mut filter_string = "";
        if filter.is_some() {
            filter_string = filter.unwrap();
            subscriber.set_subscribe(filter_string.as_bytes())?;
        } else {
            subscriber.set_subscribe(&vec![])?;
        }

        log::info!("Subscribed and beginning processing stream");

        let mut total_temp = 0;
        for _ in 0..100_000_000 {
            let string = subscriber.recv_string(0)??;
            log::trace!("Processing: {}", string);
            let chks: Vec<i64> = string.split(' ').map(|s| { s.parse().unwrap() }).collect();
            let (_zipcode, temperature, _relhumidity) = (chks[0], chks[1], chks[2]);
            total_temp += temperature;
        }

        log::info!("Average temperature for zipcode '{}' was {}F", filter_string, (total_temp / 100));
 
        Ok(())
    }

    fn wuserver(endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting server for {}", endpoint);

        let ctx = zmq::Context::new();
        let publisher = ctx.socket(zmq::PUB)?;
        publisher.connect(endpoint)?;

        let mut rng = rand::thread_rng();

        loop {
            let zipcode = rng.gen_range(0, 100_000);
            let temperature = rng.gen_range(-100, 100);
            let relhumidity = rng.gen_range(10, 50);

            let update = format!("{:05} {} {}", zipcode, temperature, relhumidity);
            publisher.send(&update, 0)?;
        }
    }
}

struct StreamFile {} 

impl StreamFile {
    fn proxy(front_endpoint: &str, back_endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting proxy for {} and {}", front_endpoint, back_endpoint);

        let ctx = zmq::Context::new();
        let frontend = ctx.socket(zmq::XSUB)?;
        let backend = ctx.socket(zmq::XPUB)?;

        frontend.bind(front_endpoint)?;
        backend.bind(back_endpoint)?;

        log::info!("Bound and beginning proxy");

        zmq::proxy(&frontend, &backend)?;

        Ok(())
    }

    fn server(endpoint: &str, file_path: &str) -> Result<(), CliError> {
        log::info!("Starting client for {} with file_path {}", endpoint, file_path);

        let ctx = zmq::Context::new();
        let publisher = ctx.socket(zmq::PUB)?;
        publisher.connect(endpoint)?;

        let mut input = std::fs::File::open(file_path)?;
        let mut byte_buffer = vec![0; 4096];
        loop {
            let bytes_read = input.read(&mut byte_buffer).unwrap_or(0);
            if bytes_read > 0 {
                publisher.send(&byte_buffer[0..bytes_read], 0)?;
            }
        }
    }

    fn client(endpoint: &str, file_path: &str) -> Result<(), CliError> {
        log::info!("Starting client for {} with file_path {}", endpoint, file_path);

        let ctx = zmq::Context::new();
        let subscriber = ctx.socket(zmq::SUB)?;
        subscriber.connect(endpoint)?;
        subscriber.set_subscribe(&vec![])?;

        log::info!("Subscribed and beginning processing stream");

        let mut output = std::fs::File::create(file_path)?;

        for _ in 0..100_000 {
            let data = subscriber.recv_bytes(0)?;
            let mut pos = 0;
            while pos < data.len() {
                pos += output.write(&data[pos..])?;
            }
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
            .arg(clap::Arg::with_name("primary_endpoint")
                .short("1")
                .long("primary_endpoint")
                .value_name("ENDPOINT")
                .help("Bind/Connect like tcp://*:0 or tcp://0.0.0.0:5555")
                .takes_value(true))
            .arg(clap::Arg::with_name("secondary_endpoint")
                .short("2")
                .long("secondary_endpoint")
                .value_name("ENDPOINT")
                .help("Bind/Connect like tcp://*:0 or tcp://0.0.0.0:5555")
                .takes_value(true))
            .arg(clap::Arg::with_name("filter")
                .short("f")
                .long("filter")
                .value_name("FILTER")
                .help("A subscribe filter if a subscriber is created")
                .takes_value(true))
            .arg(clap::Arg::with_name("file_path")
                .short("p")
                .long("file-path")
                .value_name("FILE_PATH")
                .help("A file path to use if the routine needs one")
                .takes_value(true))
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
            Some("server") => {
                (zmq::PUB, "server")
            },
            Some("client") => {
                (zmq::SUB, "client")
            },
            Some("proxy") => {
                (zmq::XPUB, "proxy")
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

        let primary_endpoint = matches.value_of("primary_endpoint");
        let secondary_endpoint= matches.value_of("secondary_endpoint");

        log::info!("Preparing {} socket for '{}' and '{}'", socket_type_name, primary_endpoint.unwrap_or(""), secondary_endpoint.unwrap_or(""));
        
        match matches.value_of("routine") {
            Some("rtdealer") => {
                match socket_type {
                    zmq::ROUTER => {
                        return RtDealer::router(primary_endpoint.unwrap());
                    },
                    zmq::DEALER => {
                        return RtDealer::dealer(primary_endpoint.unwrap());
                    },
                    _ => {
                        let routine = "rtdealer";
                        log::error!("Invalid socket type {} for routine {}", socket_type_name, routine);
                        panic!("Cannot procede");
                    }
                }
            },
            Some("wuproxy") => {
                match socket_type_name {
                    "proxy" => {
                        return WuProxy::wuproxy(primary_endpoint.unwrap(), secondary_endpoint.unwrap());
                    },
                    "server" => {
                        return WuProxy::wuserver(primary_endpoint.unwrap());
                    },
                    "client" => {
                        return WuProxy::wuclient(primary_endpoint.unwrap(), matches.value_of("filter"));
                    },
                    _ => {
                        let routine = "WuProxy";
                        log::error!("Invalid socket type {} for routine {}", socket_type_name, routine);
                        panic!("Cannot procede");
                    }
                }
            },
            Some("streamfile") => {
                match socket_type_name {
                    "proxy" => {
                        return StreamFile::proxy(primary_endpoint.unwrap(), secondary_endpoint.unwrap());
                    },
                    "server" => {
                        return StreamFile::server(primary_endpoint.unwrap(), matches.value_of("file_path").unwrap());
                    },
                    "client" => {
                        return StreamFile::client(primary_endpoint.unwrap(), matches.value_of("file_path").unwrap());
                    },
                    _ => {
                        let routine = "StreamFile";
                        log::error!("Invalid socket type {} for routine {}", socket_type_name, routine);
                        panic!("Cannot procede");
                    }
                }
            },            
            Some(routine) => {
                log::error!("Unknown routine {} for socket type {}", routine, socket_type_name);
            },
            _ => {
                log::error!("Routine must be specified");
            }
        }
    }

    Ok(())
}
