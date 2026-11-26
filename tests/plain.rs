//! A runner reaches a model on this machine over plain HTTP (#336): an entry of `reach` may be `http://<host>:<port>`,
//! on this machine alone, and the proxy lets through a plain request in absolute form to it, sent on in origin form.
//!
//! The servers are small HTTP servers on this machine's loopback, made by the test and ended by it. A session in a
//! view is a bash script that speaks through `/dev/tcp`, so no client program is needed. A test that needs bubblewrap
//! checks first that it can make a sandbox, as the tests of the view do. Nothing here reaches beyond this machine.

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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// A small HTTP server on this machine's loopback: it reads each request's head and the body its `Content-Length`
/// gives, keeps what it read, answers `200` with the body `hello <name>`, and closes. It ends with the value.
struct Server {
    port: u16,
    got: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn new(name: &str) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let got = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (g, s, body) = (got.clone(), stop.clone(), format!("hello {}", name));
        let thread = std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let read = request_of(&stream);
                        g.lock().unwrap().push(read);
                        let mut stream = stream;
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
        Server { port, got, stop, thread: Some(thread) }
    }

    /// The requests it read, each whole.
    fn got(&self) -> Vec<String> {
        self.got.lock().unwrap().iter().map(|bytes| String::from_utf8_lossy(bytes).to_string()).collect()
    }

    fn count(&self) -> usize {
        self.got.lock().unwrap().len()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A request's head and the body its `Content-Length` gives, read within ten seconds.
fn request_of(stream: &TcpStream) -> Vec<u8> {
    let mut stream = stream;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut got = Vec::new();
    let mut byte = [0u8; 1];
    while !got.ends_with(b"\r\n\r\n") && got.len() < 65536 {
        match stream.read(&mut byte) {
            Ok(1) => got.push(byte[0]),
            _ => return got,
        }
    }
    let head = String::from_utf8_lossy(&got).to_ascii_lowercase();
    let length: usize = head.lines().find_map(|line| line.strip_prefix("content-length:")).and_then(|value| value.trim().parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; length];
    if stream.read_exact(&mut body).is_ok() {
        got.extend_from_slice(&body);
    }
    got
}

/// An entry of `reach`, as the organisation's file has it.
fn entry(text: &str) -> Endpoint {
    reach::entry(text).unwrap_or_else(|why| panic!("{}: {}", text, why))
}

/// A proxy on a folder of its own, letting through `reach`, with its refusals written to `refused` of `folder`.
struct Running {
    folder: Folder,
    proxy: Option<Proxy>,
}

impl Running {
    fn new(reach: &[&str]) -> Running {
        let folder = folder_named("out");
        let endpoints: Vec<Endpoint> = reach.iter().map(|text| entry(text)).collect();
        let proxy = Proxy::start(&folder.path.join("net"), &endpoints, Some(&folder.path.join("refused"))).unwrap();
        Running { folder, proxy: Some(proxy) }
    }

    fn connect(&self) -> UnixStream {
        let stream = UnixStream::connect(self.folder.path.join("net").join("proxy")).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        stream
    }

    fn refused(&self) -> String {
        fs::read_to_string(self.folder.path.join("refused")).unwrap_or_default()
    }

    /// Sends `request` and gives all that comes back, until the other side closes.
    fn asked(&self, request: &[u8]) -> String {
        let mut stream = self.connect();
        let _ = stream.write_all(request);
        let mut back = Vec::new();
        let _ = stream.read_to_end(&mut back);
        String::from_utf8_lossy(&back).to_string()
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

// ---- WB-VIEW-153 and 154: the form of a plain endpoint ----

const ORG: &str = "levels: [public, internal]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: [session]
";

/// The runner's `reach`, as the one loader reads it from an organisation's file with `line`; a refusal names the file.
fn read_reach(line: &str) -> Result<Vec<String>, String> {
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    fs::write(&path, format!("{}    {}\n", ORG, line)).unwrap();
    watchbill::confidentiality::read(&path).map(|found| found.runners[0].reach.iter().map(|one| one.to_string()).collect()).map_err(|why| {
        assert!(why.contains(path.to_str().unwrap()), "{}: {}", line, why);
        why
    })
}

#[test]
fn test_WB_VIEW_153_an_entry_of_reach_may_be_http_with_a_host_and_a_port_and_is_marked_plain() {
    assert_eq!(
        read_reach("reach: [api.anthropic.com:443, 'http://localhost:11434', http://127.0.0.1:8080, 'http://[::1]:11434', 'http://LocalHost:9000']").unwrap(),
        ["api.anthropic.com:443", "http://localhost:11434", "http://127.0.0.1:8080", "http://[::1]:11434", "http://localhost:9000"]
    );
    assert_eq!(read_reach("reach:\n      - http://localhost:11434\n").unwrap(), ["http://localhost:11434"]);
    assert!(entry("http://localhost:11434").is_plain());
    assert!(!entry("localhost:11434").is_plain());
    assert_eq!(entry("http://localhost:11434").host(), "localhost");
    assert_eq!(entry("http://localhost:11434").port(), 11434);
}

#[test]
fn test_WB_VIEW_153_https_a_path_a_missing_port_or_another_form_of_http_is_refused_with_the_file_and_the_place() {
    for bad in [
        "reach: ['https://localhost:11434']",
        "reach: ['https://api.anthropic.com:443']",
        "reach: ['HTTP://localhost:11434']",
        "reach: ['Http://localhost:11434']",
        "reach: ['ftp://localhost:21']",
        "reach: ['http://localhost']",
        "reach: ['http://localhost:']",
        "reach: ['http://localhost:11434/']",
        "reach: ['http://localhost:11434/api']",
        "reach: ['http://localhost:11434?x=1']",
        "reach: ['http://localhost:11434#top']",
        "reach: ['http://user@localhost:11434']",
        "reach: ['http://user:secret@localhost:11434']",
        "reach: ['http://']",
        "reach: ['http:/localhost:11434']",
        "reach: ['http//localhost:11434']",
        "reach: ['http://localhost:0']",
        "reach: ['http://localhost:011434']",
        "reach: ['http://::1:11434']",
        "reach: [' http://localhost:11434']",
        "reach: ['http://localhost:11434', 'localhost:11434']",
        "reach: ['localhost:11434', 'http://LOCALHOST:11434']",
        "reach: ['http://localhost:11434', 'http://localhost:11434']",
    ] {
        let why = read_reach(bad).expect_err(bad);
        assert!(why.contains("reach"), "{}: {}", bad, why);
        assert!(why.contains("runners.stand-in.reach."), "{}: {}", bad, why);
    }
    let why = read_reach("reach: ['https://localhost:11434']").unwrap_err();
    assert!(why.contains("https://") && why.contains("CONNECT"), "{}", why);
}

#[test]
fn test_WB_VIEW_154_a_plain_endpoint_is_taken_on_this_machine_alone() {
    for bad in [
        "reach: ['http://api.example.org:80']",
        "reach: ['http://model.local:11434']",
        "reach: ['http://192.168.1.20:11434']",
        "reach: ['http://10.0.0.5:11434']",
        "reach: ['http://127.0.0.2:11434']",
        "reach: ['http://127.1.2.3:11434']",
        "reach: ['http://0.0.0.0:11434']",
        "reach: ['http://[::]:11434']",
        "reach: ['http://[::ffff:127.0.0.1]:11434']",
        "reach: ['http://[fe80::1]:11434']",
        "reach: ['http://localhost.example.org:11434']",
    ] {
        let why = read_reach(bad).expect_err(bad);
        assert!(why.contains("localhost") && why.contains("127.0.0.1") && why.contains("[::1]"), "{}: {}", bad, why);
        assert!(reach::entry(&bad[9..bad.len() - 2]).is_err(), "{}", bad);
    }
    // An endpoint by CONNECT is taken anywhere, as before.
    assert_eq!(read_reach("reach: ['192.168.1.20:11434']").unwrap(), ["192.168.1.20:11434"]);
}

#[test]
fn test_WB_VIEW_154_the_proxy_refuses_plain_http_to_a_host_off_this_machine() {
    // The host is in `reach` by CONNECT alone: a plain request to it is refused, and nothing is resolved or reached.
    let mut running = Running::new(&["api.example.org:80", "192.168.1.20:11434"]);

    for target in ["api.example.org:80", "192.168.1.20:11434"] {
        let back = running.asked(format!("GET http://{target}/ HTTP/1.1\r\nHost: {target}\r\n\r\n").as_bytes());
        assert!(back.starts_with("HTTP/1.1 403 "), "{}: {}", target, back);
        assert!(back.contains("\r\n\r\nwb: "), "{}: {}", target, back);
    }
    assert_eq!(running.refused().lines().count(), 2, "{}", running.refused());
    assert_eq!(running.stop(), 2);
}

// ---- WB-VIEW-155 and 156: what the proxy lets through to a plain endpoint ----

#[test]
fn test_WB_VIEW_155_a_plain_request_to_a_plain_endpoint_passes_in_origin_form_and_is_answered() {
    let server = Server::new("model");
    let port = server.port;
    let mut running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);

    let back = running.asked(
        format!("POST http://127.0.0.1:{port}/api/chat?stream=false HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUser-Agent: test\r\nContent-Length: 10\r\n\r\n{{\"a\": \"b\"}}")
            .as_bytes(),
    );

    assert!(back.starts_with("HTTP/1.1 200 OK\r\n"), "{}", back);
    assert!(back.ends_with("\r\n\r\nhello model"), "{}", back);
    assert_eq!(
        server.got(),
        [format!("POST /api/chat?stream=false HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUser-Agent: test\r\nContent-Length: 10\r\n\r\n{{\"a\": \"b\"}}")]
    );
    assert_eq!(running.refused(), "");
    assert_eq!(running.stop(), 0);
}

#[test]
fn test_WB_VIEW_155_a_name_of_a_plain_endpoint_is_compared_without_case_and_reached_on_loopback() {
    let server = Server::new("local");
    let port = server.port;
    let running = Running::new(&[&format!("http://localhost:{}", port)]);

    let back = running.asked(format!("GET http://LocalHost:{port}/api/tags HTTP/1.0\r\nhost: localhost:{port}\r\n\r\n").as_bytes());

    // Where `localhost` leads to `::1` first and nothing listens there, the next address of loopback is tried.
    assert!(back.starts_with("HTTP/1.1 200 OK\r\n") && back.ends_with("hello local"), "{}", back);
    assert_eq!(server.got(), [format!("GET /api/tags HTTP/1.0\r\nhost: localhost:{port}\r\n\r\n")]);
    assert_eq!(running.refused(), "");
}

#[test]
fn test_WB_VIEW_155_the_head_of_a_plain_request_is_read_under_the_same_limits() {
    let server = Server::new("model");
    let port = server.port;
    let running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);

    let long = format!("GET http://127.0.0.1:{port}/ HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX: {}\r\n\r\n", "a".repeat(9000));
    let back = running.asked(long.as_bytes());
    assert!(back.starts_with("HTTP/1.1 403 ") && back.contains("8 KiB"), "{}", back);

    // A head that does not end within ten seconds.
    let mut stream = running.connect();
    write!(stream, "GET http://127.0.0.1:{port}/ HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n").unwrap();
    let began = Instant::now();
    let mut back = Vec::new();
    let _ = stream.read_to_end(&mut back);
    let back = String::from_utf8_lossy(&back).to_string();
    assert!(back.starts_with("HTTP/1.1 403 ") && back.contains("10 seconds"), "{}", back);
    assert!(began.elapsed() >= Duration::from_secs(9) && began.elapsed() < Duration::from_secs(20), "{:?}", began.elapsed());
    assert_eq!(server.count(), 0);
}

#[test]
fn test_WB_VIEW_156_connect_to_a_plain_endpoint_is_let_through() {
    let server = Server::new("model");
    let port = server.port;
    let running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);
    let mut stream = running.connect();

    write!(stream, "CONNECT 127.0.0.1:{port} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\nGET /api/tags HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").unwrap();
    let mut back = Vec::new();
    let _ = stream.read_to_end(&mut back);
    let back = String::from_utf8_lossy(&back).to_string();

    assert!(back.starts_with("HTTP/1.1 200 Connection established\r\n\r\nHTTP/1.1 200 OK\r\n"), "{}", back);
    assert!(back.ends_with("hello model"), "{}", back);
    assert_eq!(server.got(), [format!("GET /api/tags HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")]);
    assert_eq!(running.refused(), "");
}

// ---- WB-VIEW-157: what the proxy refuses ----

/// Asks `running` for `request`, and holds that it is refused with `403` and a line `wb: …` that holds `says`.
fn refused_with(running: &Running, request: &str, says: &str) {
    let back = running.asked(request.as_bytes());
    assert!(back.starts_with("HTTP/1.1 403 Forbidden\r\n"), "{:?}: {}", request, back);
    let body = back.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default();
    assert!(body.starts_with("wb: ") && body.ends_with('\n') && body.lines().count() == 1, "{:?}: {:?}", request, body);
    assert!(body.contains(says), "{:?}: {:?} does not say {:?}", request, body, says);
}

#[test]
fn test_WB_VIEW_157_a_plain_request_to_an_endpoint_marked_for_connect_alone_is_refused() {
    let server = Server::new("model");
    let port = server.port;
    let mut running = Running::new(&[&format!("127.0.0.1:{}", port)]);

    refused_with(&running, &format!("GET http://127.0.0.1:{port}/api/tags HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"), "CONNECT");

    assert_eq!(server.count(), 0);
    assert!(running.refused().starts_with(&format!("GET http://127.0.0.1:{port}/api/tags HTTP/1.1: ")), "{}", running.refused());
    assert_eq!(running.stop(), 1);
}

#[test]
fn test_WB_VIEW_157_a_plain_request_to_a_host_or_port_not_in_reach_is_refused() {
    let server = Server::new("model");
    let other = Server::new("database");
    let (port, elsewhere) = (server.port, other.port);
    let mut running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);

    for target in [format!("127.0.0.1:{elsewhere}"), format!("localhost:{port}"), format!("127.0.0.2:{port}"), format!("[::1]:{port}"), format!("api.example.org:{port}")] {
        refused_with(&running, &format!("GET http://{target}/ HTTP/1.1\r\nHost: {target}\r\n\r\n"), "reach");
    }

    assert_eq!(server.count(), 0);
    assert_eq!(other.count(), 0);
    assert_eq!(running.stop(), 5);
}

#[test]
fn test_WB_VIEW_157_a_host_header_that_differs_from_the_target_is_refused() {
    let server = Server::new("model");
    let port = server.port;
    let mut running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);
    let line = format!("GET http://127.0.0.1:{port}/ HTTP/1.1\r\n");

    for (head, says) in [
        (format!("Host: localhost:{port}\r\n"), "Host"),
        (format!("Host: 127.0.0.1:{}\r\n", port + 1), "Host"),
        ("Host: api.example.org\r\n".to_string(), "Host"),
        ("Host: 127.0.0.1\r\n".to_string(), "Host"),
        (String::new(), "Host"),
        (format!("Host: 127.0.0.1:{port}\r\nHost: 127.0.0.1:{port}\r\n"), "Host"),
        (format!("Host: 127.0.0.1:{port}\r\nhost: api.example.org:80\r\n"), "Host"),
        (format!("X-A: 1\r\n Host: 127.0.0.1:{port}\r\n"), "folded"),
        (format!("Host: 127.0.0.1:{port}\r\nX-A 1\r\n"), "colon"),
        (format!("Host : 127.0.0.1:{port}\r\n"), "colon"),
    ] {
        refused_with(&running, &format!("{line}{head}\r\n"), says);
    }

    assert_eq!(server.count(), 0);
    assert_eq!(running.stop(), 10);
}

#[test]
fn test_WB_VIEW_157_a_request_not_in_absolute_form_with_a_port_and_a_path_is_refused() {
    let server = Server::new("model");
    let port = server.port;
    let mut running = Running::new(&[&format!("http://127.0.0.1:{}", port)]);
    let host = format!("Host: 127.0.0.1:{port}\r\n\r\n");

    for line in [
        "GET /api/tags HTTP/1.1".to_string(),
        "GET http://127.0.0.1/api/tags HTTP/1.1".to_string(),
        format!("GET http://127.0.0.1:{port} HTTP/1.1"),
        format!("GET http://127.0.0.1:{port}?x HTTP/1.1"),
        format!("GET https://127.0.0.1:{port}/ HTTP/1.1"),
        format!("GET HTTP://127.0.0.1:{port}/ HTTP/1.1"),
        format!("GET http://user@127.0.0.1:{port}/ HTTP/1.1"),
        format!("GET http://127.0.0.1:{port}/#top HTTP/1.1"),
        format!("GET http://127.0.0.1:{port}/ SPDY/3"),
        format!("get http://127.0.0.1:{port}/ HTTP/1.1"),
        format!("GET  http://127.0.0.1:{port}/ HTTP/1.1"),
        format!("GET http://127.0.0.1:{port}/\x7f HTTP/1.1"),
    ] {
        let back = running.asked(format!("{line}\r\n{host}").as_bytes());
        assert!(back.starts_with("HTTP/1.1 403 Forbidden\r\n") && back.contains("\r\n\r\nwb: "), "{:?}: {}", line, back);
    }

    assert_eq!(server.count(), 0);
    assert_eq!(running.refused().lines().count(), 12, "{}", running.refused());
    assert_eq!(running.stop(), 12);
}

// ---- WB-VIEW-158: a session in a view reaches a plain endpoint through the proxy ----

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

    /// Runs it, with a proxy that lets through `reach`.
    fn run(&self, reach: &[&str]) -> (i32, usize) {
        let endpoints: Vec<Endpoint> = reach.iter().map(|text| entry(text)).collect();
        let folder = self.out.path.join("net");
        let proxy = Proxy::start(&folder, &endpoints, Some(&self.log.path.join("refused"))).unwrap();
        let net = Net { folder: folder.clone(), exe: PathBuf::from(env!("CARGO_BIN_EXE_wb")) };
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
        let status = watchbill::view::run_reaching(Path::new(watchbill::view::BWRAP), &plan, watchbill::view::Layout::Listed, Some(&self.out.path), Some(&net), None, "wb start build", &mut std::io::sink()).unwrap();
        let refused = proxy.stop();
        (status.code().unwrap_or(-1), refused)
    }

    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.log.path.join(name)).unwrap_or_default()
    }
}

#[test]
fn test_WB_VIEW_158_a_session_reaches_a_plain_endpoint_through_the_proxy_and_no_other_way() {
    if !sandbox("test_WB_VIEW_158") {
        return;
    }
    let server = Server::new("model");
    let port = server.port;
    let session = Session::new(&format!(
        "if (exec 3<>/dev/tcp/127.0.0.1/{port}) 2>/dev/null; then echo yes > {{log}}/direct; else echo no > {{log}}/direct; fi\n\
         env > {{log}}/env\n\
         proxy=${{http_proxy#http://}}; host=${{proxy%:*}}; at=${{proxy##*:}}\n\
         if exec 3<>/dev/tcp/$host/$at; then\n\
           printf 'GET http://127.0.0.1:{port}/api/tags HTTP/1.1\\r\\nHost: 127.0.0.1:{port}\\r\\n\\r\\n' >&3\n\
           read -r -t 15 status <&3; echo \"$status\" | tr -d '\\r' > {{log}}/status\n\
           while read -r -t 15 line <&3 && [ \"$line\" != $'\\r' ] && [ -n \"$line\" ]; do :; done\n\
           read -r -t 15 body <&3; echo \"$body\" > {{log}}/body\n\
           exec 3<&-\n\
         else echo none > {{log}}/status; fi\n"
    ));

    let (status, refused) = session.run(&[&format!("http://127.0.0.1:{}", port)]);

    assert_eq!(status, 0);
    assert_eq!(session.left("direct"), "no\n");
    assert_eq!(session.left("status"), "HTTP/1.1 200 OK\n");
    assert_eq!(session.left("body"), "hello model\n");
    assert_eq!(server.got(), [format!("GET /api/tags HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")]);
    assert_eq!(refused, 0);
    let env = session.left("env");
    for name in ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"] {
        assert!(env.lines().any(|line| line == format!("{}=http://127.0.0.1:3128", name)), "{}: {}", name, env);
    }
    for name in ["NO_PROXY", "no_proxy", "ALL_PROXY", "all_proxy"] {
        assert!(!env.lines().any(|line| line.starts_with(&format!("{}=", name))), "{}: {}", name, env);
    }
}

// ---- WB-VIEW-160: the README ----

#[test]
fn test_WB_VIEW_160_the_readme_says_a_plain_endpoint_and_goose_local_has_its_reach() {
    let readme = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"));
    let start = readme.find("## The view").unwrap();
    let view = &readme[start..start + readme[start..].find("### Seeing a view first").unwrap()];
    for words in ["`http://<host>:<port>`", "not encrypted", "`localhost`, `127.0.0.1` or `[::1]`", "`https://`"] {
        assert!(view.contains(words), "the view's section does not say {:?}", words);
    }
    let start = readme.find("  goose-local:").expect("the README has no runner goose-local");
    let runner = &readme[start..start + readme[start..].find("```").unwrap()];
    assert!(runner.contains("    reach: [http://localhost:11434]\n"), "{}", runner);
    // The runner's lines are read by the one loader as they are written.
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    fs::write(&path, format!("levels: [public, internal, confidential]\nrunners:\n{}", runner)).unwrap();
    let found = watchbill::confidentiality::read(&path).unwrap();
    assert_eq!(found.runners[0].reach.iter().map(ToString::to_string).collect::<Vec<_>>(), ["http://localhost:11434"]);
}
