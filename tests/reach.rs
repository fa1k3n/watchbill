//! A session in a view reaches only its runner's endpoints (#195): the sandbox has a network of its own, and `wb` runs
//! a proxy outside it that lets through only `reach` of the runner, by `CONNECT`.
//!
//! The services are stand-ins on this machine's loopback, made by the test and ended by it. A session is a bash script
//! that speaks to them through `/dev/tcp`, so no client program is needed. A test that needs bubblewrap checks first
//! that it can make a sandbox, as the tests of the view do. Nothing here reaches beyond this machine.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use watchbill::reach::{self, Endpoint, Net, Proxy};

/// Whether bubblewrap can make a sandbox on this machine, by the check `wb` makes itself.
fn sandbox(test: &str) -> bool {
    match watchbill::view::works(Path::new(watchbill::view::BWRAP)) {
        Ok(()) => true,
        Err(why) if std::env::var_os("WB_TEST_REQUIRE_SANDBOX").is_some_and(|value| value == "1") => {
            panic!("{}: WB_TEST_REQUIRE_SANDBOX=1, and bubblewrap cannot make a sandbox here: {}", test, why)
        }
        Err(why) => {
            eprintln!("{}: nothing checked, bubblewrap cannot make a sandbox here: {}", test, why);
            false
        }
    }
}

/// A service on this machine's loopback that writes `stand-in <name>` to each connection and keeps it open until the
/// other side closes it. It counts its connections, and ends with the value.
struct StandIn {
    port: u16,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    held: Arc<Mutex<Vec<TcpStream>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl StandIn {
    fn new(name: &str) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let held = Arc::new(Mutex::new(Vec::new()));
        let (c, s, h, said) = (count.clone(), stop.clone(), held.clone(), format!("stand-in {}\n", name));
        let thread = std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        c.fetch_add(1, Ordering::SeqCst);
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.write_all(said.as_bytes());
                        h.lock().unwrap().push(stream);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
        StandIn { port, count, stop, held, thread: Some(thread) }
    }

    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        for stream in self.held.lock().unwrap().drain(..) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn endpoint(text: &str) -> Endpoint {
    reach::endpoint(text).unwrap_or_else(|why| panic!("{}: {}", text, why))
}

/// A proxy on a folder of its own, letting through `reach`, with its refusals written to `refused` of `folder`.
struct Running {
    folder: Folder,
    proxy: Option<Proxy>,
}

impl Running {
    fn new(reach: &[&str]) -> Running {
        let folder = folder_named("out");
        let endpoints: Vec<Endpoint> = reach.iter().map(|text| endpoint(text)).collect();
        let proxy = Proxy::start(&folder.path.join("net"), &endpoints, Some(&folder.path.join("refused"))).unwrap();
        Running { folder, proxy: Some(proxy) }
    }

    fn socket(&self) -> PathBuf {
        self.folder.path.join("net").join("proxy")
    }

    fn connect(&self) -> UnixStream {
        let stream = UnixStream::connect(self.socket()).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        stream
    }

    fn refused(&self) -> String {
        fs::read_to_string(self.folder.path.join("refused")).unwrap_or_default()
    }

    fn stop(&mut self) -> usize {
        self.proxy.take().map(Proxy::stop).unwrap_or(0)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The head of the proxy's answer: up to the empty line, read a byte at a time.
fn head_of(stream: &mut UnixStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&head).to_string()
}

/// What follows the head, until the proxy closes.
fn rest_of(stream: &mut UnixStream) -> String {
    let mut rest = Vec::new();
    let _ = stream.read_to_end(&mut rest);
    String::from_utf8_lossy(&rest).to_string()
}

/// One line read after the head.
fn line_after(stream: &mut UnixStream) -> String {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while !line.ends_with(b"\n") {
        match stream.read(&mut byte) {
            Ok(1) => line.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&line).to_string()
}

// ---- WB-VIEW-144: the form of `reach` ----

const ORG: &str = "levels: [public, internal]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: [session]
";

fn read_reach(line: &str) -> Result<Vec<String>, String> {
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    fs::write(&path, format!("{}    {}\n", ORG, line)).unwrap();
    watchbill::confidentiality::read(&path)
        .map(|found| found.runners[0].reach.iter().map(|one| one.to_string()).collect())
        .map_err(|why| {
            assert!(why.contains(path.to_str().unwrap()), "{}: {}", line, why);
            why
        })
}

#[test]
fn test_WB_VIEW_144_reach_is_a_list_of_hosts_and_ports_read_through_the_loader() {
    assert_eq!(
        read_reach("reach: [api.anthropic.com:443, 127.0.0.1:11434, '[::1]:8080', Model-Host.Example:8443]").unwrap(),
        ["api.anthropic.com:443", "127.0.0.1:11434", "[::1]:8080", "model-host.example:8443"]
    );
    assert_eq!(read_reach("reach: []").unwrap(), Vec::<String>::new());
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    fs::write(&path, ORG).unwrap();
    assert!(watchbill::confidentiality::read(&path).unwrap().runners[0].reach.is_empty());
}

#[test]
fn test_WB_VIEW_144_a_reach_that_is_not_a_list_of_endpoints_is_refused_with_the_file_and_the_place() {
    for bad in [
        "reach: api.anthropic.com:443",
        "reach: [443]",
        "reach: [api.anthropic.com]",
        "reach: ['api.anthropic.com:']",
        "reach: ['api.anthropic.com:0']",
        "reach: ['api.anthropic.com:65536']",
        "reach: ['api.anthropic.com:0443']",
        "reach: ['api.anthropic.com:+443']",
        "reach: ['https://api.anthropic.com:443']",
        "reach: ['api.anthropic.com:443/v1']",
        "reach: ['user@api.anthropic.com:443']",
        "reach: ['*.anthropic.com:443']",
        "reach: ['api.anthropic.com.:443']",
        "reach: ['-api.anthropic.com:443']",
        "reach: ['api..anthropic.com:443']",
        "reach: ['api_x.anthropic.com:443']",
        "reach: [':443']",
        "reach: ['::1:443']",
        "reach: ['[::1]']",
        "reach: ['[not-an-address]:443']",
        "reach: ['api.anthropic.com:443-445']",
        "reach: ['api.anthropic.com :443']",
        "reach: ['']",
        "reach: [api.anthropic.com:443, API.anthropic.com:443]",
        "reach: [{host: api.anthropic.com}]",
        "reach: [[api.anthropic.com:443]]",
        "reach: {api.anthropic.com: 443}",
    ] {
        let why = read_reach(bad).expect_err(bad);
        assert!(why.contains("reach"), "{}: {}", bad, why);
    }
    let long = format!("reach: ['{}.example:443']", "a".repeat(64));
    assert!(read_reach(&long).is_err());
}

// ---- WB-VIEW-143, 145 and 146: the words bubblewrap is given ----

fn plan_words(net: Option<&Net>) -> Vec<String> {
    let words = ["runner".to_string(), "--flag".to_string()];
    let plan = watchbill::view::Plan {
        view: Path::new("/var/views/1"),
        checkout: Path::new("/work/brakes"),
        binds: vec![],
        organisation: None,
        home: Some(Path::new("/home/ada")),
        new_session: false,
        words: &words,
    };
    let out = Path::new("/state/watchbill/out/1");
    watchbill::view::arguments_reaching(&plan, watchbill::view::Layout::Listed, Some(out), net)
        .iter()
        .map(|word| word.to_str().unwrap().to_string())
        .collect()
}

/// Where in `words` the run of `pair` starts.
fn find(words: &[String], pair: &[&str]) -> Option<usize> {
    words.windows(pair.len()).position(|run| run == pair)
}

#[test]
fn test_WB_VIEW_143_the_sandbox_has_a_network_of_its_own() {
    for net in [None, Some(&Net { folder: PathBuf::from("/state/watchbill/out/1/net"), exe: PathBuf::from("/usr/bin/wb") })] {
        let words = plan_words(net);
        assert!(words.iter().any(|word| word == "--unshare-net"), "{:?}", words);
        assert!(!words.iter().any(|word| word == "--share-net" || word == "--unshare-all"), "{:?}", words);
    }
    // So does a view of the paths of an issue's file with no `out`, as `view::arguments` gives it.
    let plan = watchbill::view::Plan {
        view: Path::new("/v"),
        checkout: Path::new("/c"),
        binds: vec![],
        organisation: None,
        home: None,
        new_session: false,
        words: &["runner".to_string()],
    };
    let words: Vec<String> = watchbill::view::arguments(&plan).iter().map(|word| word.to_str().unwrap().to_string()).collect();
    assert!(words.iter().any(|word| word == "--unshare-net"), "{:?}", words);
}

#[test]
fn test_WB_VIEW_145_with_no_reach_there_is_no_forwarder_and_every_variable_of_a_proxy_is_taken_out() {
    let words = plan_words(None);
    for name in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy"] {
        assert!(find(&words, &["--unsetenv", name]).is_some(), "{}: {:?}", name, words);
    }
    assert!(!words.iter().any(|word| word == "--setenv"), "{:?}", words);
    assert!(!words.iter().any(|word| word.contains(reach::FORWARDER) || word.contains(reach::NET_FOLDER)), "{:?}", words);
    let rest: Vec<&str> = words.iter().skip_while(|word| *word != "--").map(String::as_str).collect();
    assert_eq!(rest, ["--", "runner", "--flag"]);
}

#[test]
fn test_WB_VIEW_146_with_reach_wb_is_bound_as_the_forwarder_and_the_runner_is_given_the_proxy() {
    let net = Net { folder: PathBuf::from("/state/watchbill/out/1/net"), exe: PathBuf::from("/opt/wb/bin/wb") };
    let words = plan_words(Some(&net));
    let socket = find(&words, &["--ro-bind", "/state/watchbill/out/1/net", "/run/watchbill/net"]).expect("the socket's folder is not bound read-only");
    let out = find(&words, &["--bind", "/state/watchbill/out/1", "/run/watchbill"]).expect("the out folder is not bound");
    assert!(out < socket, "{:?}", words);
    assert!(find(&words, &["--ro-bind", "/opt/wb/bin/wb", "/run/watchbill-forward"]).is_some(), "{:?}", words);
    for name in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        assert!(find(&words, &["--setenv", name, "http://127.0.0.1:3128"]).is_some(), "{}: {:?}", name, words);
        assert!(find(&words, &["--unsetenv", name]).is_none(), "{}: {:?}", name, words);
    }
    for name in ["ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy"] {
        assert!(find(&words, &["--unsetenv", name]).is_some(), "{}: {:?}", name, words);
    }
    let rest: Vec<&str> = words.iter().skip_while(|word| *word != "--").map(String::as_str).collect();
    assert_eq!(rest, ["--", "/run/watchbill-forward", "--", "runner", "--flag"]);
    assert_eq!(reach::FORWARDER, "/run/watchbill-forward");
    assert_eq!(reach::SOCKET, "/run/watchbill/net/proxy");
    assert_eq!(reach::PORT, 3128);
}

// ---- WB-VIEW-147: the proxy lets through CONNECT to an endpoint of `reach` ----

#[test]
fn test_WB_VIEW_147_connect_to_an_endpoint_of_reach_is_let_through_both_ways() {
    let service = StandIn::new("model");
    let running = Running::new(&[&format!("127.0.0.1:{}", service.port)]);
    let mut stream = running.connect();

    write!(stream, "CONNECT 127.0.0.1:{} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n", service.port, service.port).unwrap();

    let head = head_of(&mut stream);
    assert!(head.starts_with("HTTP/1.1 200 "), "{}", head);
    assert_eq!(line_after(&mut stream), "stand-in model\n");
    assert_eq!(service.count(), 1);
    assert_eq!(running.refused(), "");
}

#[test]
fn test_WB_VIEW_147_a_name_is_compared_without_case_and_resolved_outside() {
    let service = StandIn::new("local");
    let running = Running::new(&[&format!("localhost:{}", service.port)]);
    let mut stream = running.connect();

    write!(stream, "CONNECT LocalHost:{} HTTP/1.0\r\n\r\n", service.port).unwrap();

    let head = head_of(&mut stream);
    assert!(head.starts_with("HTTP/1.1 200 "), "{}", head);
    assert_eq!(line_after(&mut stream), "stand-in local\n");
}

#[test]
fn test_WB_VIEW_147_bytes_sent_with_the_head_are_passed_on() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let running = Running::new(&[&format!("127.0.0.1:{}", port)]);
    let mut stream = running.connect();

    write!(stream, "CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\nhello early\n", port).unwrap();
    let (mut far, _) = listener.accept().unwrap();
    far.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let mut got = [0u8; 12];
    far.read_exact(&mut got).unwrap();
    far.write_all(b"back\n").unwrap();

    assert_eq!(&got, b"hello early\n");
    let head = head_of(&mut stream);
    assert!(head.starts_with("HTTP/1.1 200 "), "{}", head);
    assert_eq!(line_after(&mut stream), "back\n");
}

// ---- WB-VIEW-148: everything else is refused ----

/// Sends `request` and gives the head and the rest of the answer.
fn asked(running: &Running, request: &[u8]) -> (String, String) {
    let mut stream = running.connect();
    let _ = stream.write_all(request);
    let head = head_of(&mut stream);
    let rest = rest_of(&mut stream);
    (head, rest)
}

#[test]
fn test_WB_VIEW_148_connect_to_an_endpoint_not_in_reach_is_refused_and_never_connected() {
    let allowed = StandIn::new("allowed");
    let other = StandIn::new("other");
    let running = Running::new(&[&format!("127.0.0.1:{}", allowed.port)]);

    for target in [format!("127.0.0.1:{}", other.port), format!("localhost:{}", allowed.port), format!("127.0.0.2:{}", allowed.port)] {
        let (head, rest) = asked(&running, format!("CONNECT {} HTTP/1.1\r\n\r\n", target).as_bytes());
        assert!(head.starts_with("HTTP/1.1 403 "), "{}: {}", target, head);
        assert!(rest.starts_with("wb: ") && rest.contains(&target) && rest.contains("reach"), "{}: {}", target, rest);
    }
    assert_eq!(other.count(), 0);
    assert_eq!(allowed.count(), 0);
}

#[test]
fn test_WB_VIEW_148_plain_http_is_refused_also_to_an_endpoint_of_reach() {
    let allowed = StandIn::new("allowed");
    let running = Running::new(&[&format!("127.0.0.1:{}", allowed.port)]);

    let (head, rest) = asked(&running, format!("GET http://127.0.0.1:{}/ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", allowed.port).as_bytes());

    assert!(head.starts_with("HTTP/1.1 403 "), "{}", head);
    assert!(rest.starts_with("wb: ") && rest.contains("CONNECT"), "{}", rest);
    assert_eq!(allowed.count(), 0);
}

#[test]
fn test_WB_VIEW_148_a_request_that_cannot_be_read_or_is_too_long_is_refused() {
    let allowed = StandIn::new("allowed");
    let running = Running::new(&[&format!("127.0.0.1:{}", allowed.port)]);

    for request in [
        b"\x16\x03\x01\x02\x00\x01\x00\x01\xfc\x03\x03\r\n\r\n".to_vec(),
        b"CONNECT\r\n\r\n".to_vec(),
        format!("CONNECT 127.0.0.1:{} SPDY/3\r\n\r\n", allowed.port).into_bytes(),
        format!("CONNECT 127.0.0.1:{} HTTP/1.1\r\nX: {}\r\n\r\n", allowed.port, "a".repeat(9000)).into_bytes(),
    ] {
        let (head, _) = asked(&running, &request);
        assert!(head.starts_with("HTTP/1.1 403 "), "{:?}: {}", String::from_utf8_lossy(&request[..request.len().min(60)]), head);
    }
    assert_eq!(allowed.count(), 0);
}

#[test]
fn test_WB_VIEW_148_no_more_than_64_connections_are_open_at_once() {
    let allowed = StandIn::new("allowed");
    let running = Running::new(&[&format!("127.0.0.1:{}", allowed.port)]);
    let mut open = Vec::new();
    for _ in 0..64 {
        let mut stream = running.connect();
        write!(stream, "CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n", allowed.port).unwrap();
        let head = head_of(&mut stream);
        assert!(head.starts_with("HTTP/1.1 200 "), "{}", head);
        open.push(stream);
    }

    let (head, rest) = asked(&running, format!("CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n", allowed.port).as_bytes());

    assert!(head.starts_with("HTTP/1.1 503 "), "{}", head);
    assert!(rest.contains("64"), "{}", rest);
    assert_eq!(allowed.count(), 64);
    // One that ends makes room again.
    drop(open.pop());
    let end = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let mut stream = running.connect();
        write!(stream, "CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n", allowed.port).unwrap();
        let head = head_of(&mut stream);
        if head.starts_with("HTTP/1.1 200 ") {
            break;
        }
        assert!(std::time::Instant::now() < end, "no room was made: {}", head);
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn test_WB_VIEW_148_an_endpoint_of_reach_that_cannot_be_connected_to_is_a_502_and_no_refusal() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut running = Running::new(&[&format!("127.0.0.1:{}", port)]);

    let (head, _) = asked(&running, format!("CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n", port).as_bytes());

    assert!(head.starts_with("HTTP/1.1 502 "), "{}", head);
    assert_eq!(running.refused(), "");
    assert_eq!(running.stop(), 0);
}

// ---- WB-VIEW-150: each refusal is a line of `refused` ----

#[test]
fn test_WB_VIEW_150_each_refusal_is_a_line_of_refused_escaped_and_counted() {
    let allowed = StandIn::new("allowed");
    let mut running = Running::new(&[&format!("127.0.0.1:{}", allowed.port)]);

    asked(&running, b"CONNECT example.org:443 HTTP/1.1\r\n\r\n");
    asked(&running, b"GET http://example.org/\x1b[31m HTTP/1.1\r\n\r\n");
    asked(&running, format!("CONNECT {}:443 HTTP/1.1\r\n\r\n", "b".repeat(300)).as_bytes());

    let refused = running.refused();
    let lines: Vec<&str> = refused.lines().collect();
    assert_eq!(lines.len(), 3, "{}", refused);
    assert!(lines[0].starts_with("CONNECT example.org:443 HTTP/1.1: ") && lines[0].contains("reach"), "{}", lines[0]);
    assert!(lines[1].starts_with("GET http://example.org/") && lines[1].contains("CONNECT"), "{}", lines[1]);
    assert!(!lines[1].contains('\x1b'), "{:?}", lines[1]);
    assert!(lines[2].chars().count() < 300, "{}", lines[2]);
    assert_eq!(running.stop(), 3);
}

#[test]
fn test_WB_VIEW_150_a_thousand_lines_are_written_at_most_and_the_count_goes_on() {
    let mut running = Running::new(&["127.0.0.1:9"]);

    for _ in 0..1005 {
        asked(&running, b"CONNECT example.org:443 HTTP/1.1\r\n\r\n");
    }

    assert_eq!(running.refused().lines().count(), 1000);
    assert_eq!(running.stop(), 1005);
}

// ---- WB-VIEW-151: the proxy ends ----

#[test]
fn test_WB_VIEW_151_the_proxy_stopped_shuts_each_open_tunnel_and_removes_its_socket() {
    let service = StandIn::new("model");
    let mut running = Running::new(&[&format!("127.0.0.1:{}", service.port)]);
    let mut stream = running.connect();
    write!(stream, "CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n", service.port).unwrap();
    assert!(head_of(&mut stream).starts_with("HTTP/1.1 200 "));
    assert_eq!(line_after(&mut stream), "stand-in model\n");

    let began = std::time::Instant::now();
    running.stop();

    assert!(began.elapsed() < Duration::from_secs(5), "{:?}", began.elapsed());
    let mut rest = Vec::new();
    assert_eq!(stream.read_to_end(&mut rest).unwrap_or(0), 0);
    assert!(!running.socket().exists());
    assert!(UnixStream::connect(running.socket()).is_err());
}

#[test]
fn test_WB_VIEW_151_the_proxy_folder_must_be_new_and_is_made_for_its_owner_alone() {
    let folder = folder_named("out");
    fs::create_dir(folder.path.join("net")).unwrap();
    assert!(Proxy::start(&folder.path.join("net"), &[endpoint("127.0.0.1:9")], None).is_err());

    let proxy = Proxy::start(&folder.path.join("new"), &[endpoint("127.0.0.1:9")], None).unwrap();
    let mode = fs::metadata(folder.path.join("new")).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    proxy.stop();
}

// ---- In the sandbox ----

/// A session in a sandbox, run by `view::run_reaching` with the forwarder of this build: `body` is a bash script, with
/// `{log}` its folder, which is bound writable.
struct Session {
    log: Folder,
    view: Folder,
    checkout: Folder,
    out: Folder,
    home: Folder,
}

impl Session {
    fn new(body: &str) -> Session {
        let session = Session { log: folder_named("log"), view: folder_named("view"), checkout: folder_named("checkout"), out: folder_named("out"), home: folder_named("home") };
        let path = session.log.path.join("session.sh");
        fs::write(&path, format!("#!/bin/bash\n{}", body.replace("{log}", session.log.path.to_str().unwrap()))).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        session
    }

    /// Runs it, with a proxy that lets through `reach` where there is any.
    fn run(&self, reach: &[&str]) -> (i32, usize) {
        let endpoints: Vec<Endpoint> = reach.iter().map(|text| endpoint(text)).collect();
        let folder = self.out.path.join("net");
        let proxy = if endpoints.is_empty() { None } else { Some(Proxy::start(&folder, &endpoints, Some(&self.log.path.join("refused"))).unwrap()) };
        let net = proxy.as_ref().map(|_| Net { folder: folder.clone(), exe: PathBuf::from(env!("CARGO_BIN_EXE_wb")) });
        let words = [self.log.path.join("session.sh").to_str().unwrap().to_string()];
        let plan = watchbill::view::Plan {
            view: &self.view.path,
            checkout: &self.checkout.path,
            binds: vec![(self.log.path.clone(), true)],
            organisation: None,
            home: Some(&self.home.path),
            new_session: true,
            words: &words,
        };
        let status = watchbill::view::run_reaching(Path::new(watchbill::view::BWRAP), &plan, watchbill::view::Layout::Listed, Some(&self.out.path), net.as_ref(), None, "wb start build", &mut std::io::sink()).unwrap();
        let refused = proxy.map(Proxy::stop).unwrap_or(0);
        (status.code().unwrap_or(-1), refused)
    }

    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.log.path.join(name)).unwrap_or_default()
    }
}

/// A bash script that tries `127.0.0.1:<port>` directly and writes `yes` or `no` to `{log}/<name>`.
fn direct(port: u16, name: &str) -> String {
    format!("if (exec 3<>/dev/tcp/127.0.0.1/{port}) 2>/dev/null; then echo yes > {{log}}/{name}; else echo no > {{log}}/{name}; fi\n")
}

/// A bash script that asks the proxy, through the forwarder, for `target` and writes the status line and the line after
/// the head to `{log}/<name>.status` and `{log}/<name>`.
fn through(target: &str, name: &str) -> String {
    format!(
        "if exec 3<>/dev/tcp/127.0.0.1/3128; then\n\
           printf 'CONNECT {target} HTTP/1.1\\r\\n\\r\\n' >&3\n\
           read -r -t 15 status <&3; echo \"$status\" | tr -d '\\r' > {{log}}/{name}.status\n\
           while read -r -t 15 line <&3 && [ \"$line\" != $'\\r' ] && [ -n \"$line\" ]; do :; done\n\
           read -r -t 15 said <&3; echo \"$said\" | tr -d '\\r' > {{log}}/{name}\n\
           exec 3<&-\n\
         else echo none > {{log}}/{name}.status; fi\n"
    )
}

#[test]
fn test_WB_VIEW_143_a_session_reaches_no_service_of_this_machine_directly() {
    if !sandbox("test_WB_VIEW_143") {
        return;
    }
    let service = StandIn::new("local");
    let session = Session::new(&direct(service.port, "direct"));

    let (status, _) = session.run(&[&format!("127.0.0.1:{}", service.port)]);

    assert_eq!(status, 0);
    assert_eq!(session.left("direct"), "no\n");
    assert_eq!(service.count(), 0);
}

#[test]
fn test_WB_VIEW_145_a_runner_with_no_reach_reaches_nothing_and_has_no_proxy() {
    if !sandbox("test_WB_VIEW_145") {
        return;
    }
    let service = StandIn::new("local");
    let session = Session::new(&format!(
        "{}{}env > {{log}}/env\n",
        direct(service.port, "direct"),
        through(&format!("127.0.0.1:{}", service.port), "through")
    ));

    let (status, refused) = session.run(&[]);

    assert_eq!(status, 0);
    assert_eq!(session.left("direct"), "no\n");
    assert_eq!(session.left("through.status"), "none\n");
    assert_eq!(service.count(), 0);
    assert_eq!(refused, 0);
    let env = session.left("env");
    assert!(!env.to_lowercase().contains("_proxy="), "{}", env);
}

#[test]
fn test_WB_VIEW_146_a_session_reaches_an_endpoint_of_reach_through_the_proxy() {
    if !sandbox("test_WB_VIEW_146") {
        return;
    }
    let service = StandIn::new("model");
    let session = Session::new(&format!("{}echo \"$HTTPS_PROXY $https_proxy $HTTP_PROXY $http_proxy\" > {{log}}/proxy\n", through(&format!("127.0.0.1:{}", service.port), "through")));

    let (status, refused) = session.run(&[&format!("127.0.0.1:{}", service.port)]);

    assert_eq!(status, 0);
    assert!(session.left("through.status").starts_with("HTTP/1.1 200 "), "{}", session.left("through.status"));
    assert_eq!(session.left("through"), "stand-in model\n");
    assert_eq!(service.count(), 1);
    assert_eq!(refused, 0);
    assert_eq!(session.left("proxy"), "http://127.0.0.1:3128 http://127.0.0.1:3128 http://127.0.0.1:3128 http://127.0.0.1:3128\n");
}

#[test]
fn test_WB_VIEW_148_a_session_is_refused_an_endpoint_not_in_reach() {
    if !sandbox("test_WB_VIEW_148") {
        return;
    }
    let allowed = StandIn::new("model");
    let other = StandIn::new("database");
    let session = Session::new(&through(&format!("127.0.0.1:{}", other.port), "through"));

    let (status, refused) = session.run(&[&format!("127.0.0.1:{}", allowed.port)]);

    assert_eq!(status, 0);
    assert!(session.left("through.status").starts_with("HTTP/1.1 403 "), "{}", session.left("through.status"));
    assert!(session.left("through").starts_with("wb: "), "{}", session.left("through"));
    assert_eq!(other.count(), 0);
    assert_eq!(refused, 1);
    assert!(session.left("refused").contains(&format!("CONNECT 127.0.0.1:{}", other.port)), "{}", session.left("refused"));
}

#[test]
fn test_WB_VIEW_146_the_session_keeps_its_status_and_the_forwarder_is_its_child() {
    if !sandbox("test_WB_VIEW_146") {
        return;
    }
    let service = StandIn::new("model");
    // `read` is a builtin of bash: the children are read with no child of its own.
    let session = Session::new("read -r kids < /proc/$$/task/$$/children\necho \"$kids\" > {log}/children\nexit 7\n");

    let (status, _) = session.run(&[&format!("127.0.0.1:{}", service.port)]);

    assert_eq!(status, 7);
    assert_eq!(session.left("children").split_whitespace().count(), 1, "{:?}", session.left("children"));
}

/// The processes of this machine whose first word is the forwarder's and whose folder is `checkout`.
fn forwarders_in(checkout: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let cmdline = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let first = cmdline.split(|byte| *byte == 0).next().unwrap_or_default();
        if first != reach::FORWARDER.as_bytes() {
            continue;
        }
        if fs::read_link(entry.path().join("cwd")).is_ok_and(|cwd| cwd == checkout) {
            found.push(name);
        }
    }
    found
}

#[test]
fn test_WB_VIEW_151_the_forwarder_ends_with_the_session() {
    if !sandbox("test_WB_VIEW_151") {
        return;
    }
    let service = StandIn::new("model");
    // The session leaves a tunnel open as it ends.
    let session = Session::new("exec 3<>/dev/tcp/127.0.0.1/3128\nprintf 'CONNECT 127.0.0.1:%s HTTP/1.1\\r\\n\\r\\n' \"$PORT\" >&3\nread -r -t 15 status <&3\necho \"$status\" > {log}/status\nexit 0\n".replace("$PORT", &service.port.to_string()).as_str());

    let (status, _) = session.run(&[&format!("127.0.0.1:{}", service.port)]);

    assert_eq!(status, 0);
    assert!(session.left("status").starts_with("HTTP/1.1 200 "), "{}", session.left("status"));
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while !forwarders_in(&session.checkout.path).is_empty() && std::time::Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(forwarders_in(&session.checkout.path), Vec::<String>::new());
}
