mod request;
mod response;

use clap::Parser;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::sync::Mutex;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Parser, Debug)]
#[command(about = "Fun with load balancing")]
struct CmdOptions {
    /// "IP/port to bind to"
    #[arg(short, long, default_value = "0.0.0.0:1100")]
    bind: String,
    /// "Upstream host to forward requests to"
    #[arg(short, long)]
    upstream: Vec<String>,
    /// "Perform active health checks on this interval (in seconds)"
    #[arg(long, default_value = "10")]
    active_health_check_interval: usize,
    /// "Path to send request to for active health checks"
    #[arg(long, default_value = "/")]
    active_health_check_path: String,
    /// "Maximum number of requests to accept per IP per minute (0 = unlimited)"
    #[arg(long, default_value = "0")]
    max_requests_per_minute: usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: RwLock<Vec<(String, bool)>>,
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections
    let state = ProxyState {
        upstream_addresses: RwLock::new(options.upstream.into_iter().map(|x| (x, true)).collect()),
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
    };
    let state = Arc::new(state);
    let state_check = state.clone();
    tokio::spawn(async move{
        active_check_intime(&state_check).await;
    });
    let hashmap: Arc<RwLock<HashMap<String, Arc<Mutex<usize>>>>> = Arc::new(RwLock::new(HashMap::new()));

    let hashmap_clone = Arc::clone(&hashmap);
    tokio::spawn(async move{
        intimeclear(&hashmap_clone).await;
    });

    while let Ok((stream, _)) = listener.accept().await {
        // Handle the connection!
        let state_clone = Arc::clone(&state);
        let client_ip = stream.peer_addr().unwrap().ip().to_string();
        let mut need_write = false;
        {
            let hashmap = hashmap.read().await;
            if !hashmap.contains_key(&client_ip) {
                need_write = true;
            }
        }
        if need_write {
            let mut hashmap = hashmap.write().await;
            hashmap.insert(client_ip.clone(), Arc::new(Mutex::new(0)));
        }
        let hashmap = hashmap.read().await;
        let limit = Arc::clone(hashmap.get(&client_ip).unwrap());
        tokio::spawn(async move{ handle_connection(stream, &state_clone, &limit).await });
    }
}

async fn active_check_intime(state: &ProxyState) {
    loop {
        // wait times
        tokio::time::sleep(tokio::time::Duration::from_secs(
            state.active_health_check_interval as u64,
        ))
        .await;
        let mut invalidip = Vec::new();
        let mut validip = Vec::new();

        // send requests to all upstream servers
        {
            let upstream = state.upstream_addresses.read().await;

            for (ip, _is_true) in upstream.iter() {
                let newstream = TcpStream::connect(ip).await;
                if newstream.is_err() {
                    invalidip.push(ip.to_string());
                    continue;
                }
                let mut newstream = newstream.unwrap();
                let body: Vec<u8> = Vec::new();
                let request = http::Request::builder()
                    .method(http::Method::GET)
                    .uri(&state.active_health_check_path)
                    .header("Host", ip)
                    .body(body)
                    .unwrap();
                match request::write_to_stream(&request, &mut newstream).await{
                    Ok(_) => (),
                    Err(_error) => {
                        invalidip.push(ip.to_string());
                        continue;
                    }
                }
                let response =
                    match response::read_from_stream(&mut newstream, request.method()).await {
                        Ok(response) => response,
                        Err(_error) => {
                            invalidip.push(ip.to_string());
                            continue;
                        }
                    };
                if response.status().as_u16() == 200 {
                    validip.push(ip.to_string());
                } else {
                    invalidip.push(ip.to_string());
                }
            }
        }
        // modify status
        {
            let mut upstream = state.upstream_addresses.write().await;
            for item in &mut *upstream {
                if invalidip.contains(&item.0) {
                    item.1 = false;
                }
                if validip.contains(&item.0) {
                    item.1 = true;
                }
            }
        }
    }
}

async fn connect_to_upstream(state: &ProxyState) -> Result<TcpStream, std::io::Error> {
    let mut stream: Result<TcpStream, std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "All upstream servers are down",
    ));
    let mut gotip: bool = false;
    let mut invalidip: Vec<String> = Vec::new();
    let mut validip: Vec<String> = Vec::new();
    {
        let upstream = state.upstream_addresses.read().await;
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut ips = upstream
            .iter()
            .filter(|&&(_, is_true)| is_true)
            .collect::<Vec<_>>();
        loop {
            if ips.len() == 0 {
                break;
            }
            let index = rng.gen_range(0..ips.len());
            let newstream = TcpStream::connect(&ips[index].0).await;
            if newstream.is_ok() {
                stream = newstream;
                gotip = true;
                break;
            }
            invalidip.push(ips[index].0.to_string());
            ips.remove(index);
        }
        if !gotip {
            for (ip, _is_true) in upstream.iter().filter(|&&(_, is_true)| !is_true) {
                let newstream = TcpStream::connect(ip).await;
                if newstream.is_ok() {
                    stream = newstream;
                    gotip = true;
                    validip.push(ip.to_string());
                    break;
                }
            }
        }
    }
    if !invalidip.is_empty() || !validip.is_empty() {
        let mut upstream = state.upstream_addresses.write().await;
        for item in &mut *upstream {
            if invalidip.contains(&item.0) {
                item.1 = false;
            }
            if validip.contains(&item.0) {
                item.1 = true;
            }
        }
    }
    if !gotip {
        log::error!("Failed to connect to upstream : No valid ip");
    }
    stream
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

// Set 0 for rate limiting
async fn intimeclear(limits:&RwLock<HashMap<String, Arc<Mutex<usize>>>> ){
    let interval = Duration::from_secs(60);
    loop{
        tokio::time::sleep(interval).await;
        {
            let hash = limits.write().await;
            for (_ip, limit) in hash.iter(){
                let mut li = limit.lock().await;
                *li = 0;
            }
        }
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: &ProxyState,limit: &Mutex<usize>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = upstream_conn.peer_addr().unwrap().ip().to_string();
    
    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };

        // Add limit 
        if state.max_requests_per_minute!=0{
            let mut li = limit.lock().await;
            *li += 1;
            if *li > state.max_requests_per_minute{
                let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
                send_response(&mut client_conn, &response).await;
                return;
            }
        }

        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!(
                "Failed to send request to upstream {}: {}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await
        {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}
