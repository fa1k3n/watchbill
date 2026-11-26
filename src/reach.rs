//! The network of a view (#195): a session reaches only its runner's endpoints, never the services of this machine or
//! the rest of the network.
//!
//! - **No network in the sandbox.** bubblewrap starts it with `--unshare-net`: inside there is only its own loopback.
//! - **A proxy outside it**, as threads of the `wb` that waits for the session, on a Unix socket in a new folder of
//!   `wb`'s own, bound read-only in the sandbox at `NET_FOLDER`. It takes `CONNECT` (and plain HTTP, below), to an
//!   endpoint of the runner's `reach` alone, and passes the bytes both ways unread. No port is opened on the machine.
//! - **The way in.** The program of `wb` is bound read-only at `FORWARDER` and is the session's first program: it
//!   listens on `127.0.0.1:PORT` of the sandbox's loopback, starts one copy of itself that passes each connection to
//!   the socket, and runs the runner's `start` in its own place. The runner is given `HTTPS_PROXY` and `HTTP_PROXY`.
//! - **A runner with no `reach`** gets none of it, and no network.
//! - **A plain endpoint** (#336), `http://<host>:<port>` in `reach`, is on this machine alone: the proxy also takes a
//!   request in absolute form to it, and sends it on in origin form. Everything else still takes `CONNECT` alone.
//!
//! The forwarder is a child of the session, in the sandbox's own process namespace, so it ends when the session ends.
//! The proxy is stopped by `wb` when bubblewrap has ended (`Proxy::stop`).

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Where the program of `wb` is bound in the sandbox, and the name it is run by there: `wb` takes the form of the
/// forwarder only where its own name is this (`main`).
pub const FORWARDER: &str = "/run/watchbill-forward";

/// The folder of the proxy's socket in the sandbox, bound read-only.
pub const NET_FOLDER: &str = "/run/watchbill/net";

/// The name of the folder of the proxy's socket, in the folder of what a session writes for `wb`.
pub const SOCKET_FOLDER: &str = "net";

/// The name of the proxy's socket in its folder.
pub const SOCKET_NAME: &str = "proxy";

/// The proxy's socket in the sandbox.
pub const SOCKET: &str = "/run/watchbill/net/proxy";

/// The port the forwarder listens on, on the sandbox's own loopback.
pub const PORT: u16 = 3128;

/// What the runner is given as its proxy.
pub const PROXY: &str = "http://127.0.0.1:3128";

/// The variables that name the proxy: given where the runner has `reach`, and taken out where it has none.
pub const GIVEN: [&str; 4] = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"];

/// The variables that would send a connection around the proxy, or name another: always taken out.
pub const AROUND: [&str; 4] = ["ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy"];

/// The word after `FORWARDER` that makes the copy that passes the connections on.
const LISTENING: &str = "--listening";

/// The most a request's head may be.
const HEAD_CAP: usize = 8192;

/// The time a request's head is read within.
const READ_WITHIN: Duration = Duration::from_secs(10);

/// The time a connection to each address of an endpoint is made within.
const CONNECT_WITHIN: Duration = Duration::from_secs(10);

/// The most connections open at once.
pub const MOST_OPEN: usize = 64;

/// The most lines of `refused` written; the count goes on.
pub const MOST_LINES: usize = 1000;

/// The most characters of a request line in `refused`.
const SHOWN: usize = 200;

/// The stack of each thread of the proxy and the forwarder: they only pass bytes on.
const STACK: usize = 128 * 1024;

/// How long `Proxy::stop` waits for the connections to end, once each is shut down.
const STOP_WITHIN: Duration = Duration::from_secs(2);

/// The scheme that marks an endpoint of `reach` plain (#336).
pub const PLAIN: &str = "http://";

/// The hosts a plain endpoint may have, as they are kept: this machine alone (#336).
pub const PLAIN_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// An endpoint of `reach`: a host and a port, and whether it is plain (#336). A name is kept in lower case, an address
/// as the standard library writes it, an IPv6 address in brackets.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint {
    host: String,
    port: u16,
    plain: bool,
}

impl Endpoint {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether it is marked plain, `http://<host>:<port>` (#336): the proxy also takes plain HTTP to it.
    pub fn is_plain(&self) -> bool {
        self.plain
    }

    /// Whether `other` has the same host and port, plain or not.
    pub fn same_place(&self, other: &Endpoint) -> bool {
        self.host == other.host && self.port == other.port
    }

    /// The host as it is resolved: an IPv6 address without its brackets.
    fn bare(&self) -> &str {
        self.host.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or(&self.host)
    }

    /// Whether the host is this machine as it is written: `localhost`, or a loopback or unspecified address.
    pub fn on_this_machine(&self) -> bool {
        if self.host == "localhost" {
            return true;
        }
        match self.bare().parse::<IpAddr>() {
            Ok(IpAddr::V4(ip)) => ip.is_loopback() || ip.is_unspecified(),
            Ok(IpAddr::V6(ip)) => ip.is_loopback() || ip.is_unspecified() || ip.to_ipv4_mapped().is_some_and(|four| four.is_loopback() || four.is_unspecified()),
            Err(_) => false,
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scheme = if self.plain { PLAIN } else { "" };
        write!(f, "{}{}:{}", scheme, self.host, self.port)
    }
}

/// Reads an entry of `reach`: `<host>:<port>`, taken by `CONNECT` (#195), or `http://<host>:<port>`, marked plain, with
/// the port written, nothing after it, and the host `localhost`, `127.0.0.1` or `[::1]` alone (#336). The error says
/// why it is not one.
pub fn entry(text: &str) -> Result<Endpoint, String> {
    if let Some(rest) = text.strip_prefix(PLAIN) {
        if rest.contains(['/', '?', '#', '@']) {
            return Err("is not `http://<host>:<port>`: a plain endpoint has no user, and nothing after the port, no `/`, path, query or fragment".to_string());
        }
        let mut found = endpoint(rest).map_err(|why| why.replacen("is not `<host>:<port>`", "is not `http://<host>:<port>`", 1))?;
        if !PLAIN_HOSTS.contains(&found.host.as_str()) {
            return Err(format!(
                "is plain HTTP to {}, and plain HTTP is taken to this machine alone: the host is `localhost`, `127.0.0.1` or `[::1]`. An endpoint elsewhere is written `<host>:<port>`, and takes CONNECT",
                found.host
            ));
        }
        found.plain = true;
        return Ok(found);
    }
    if let Some((scheme, _)) = text.split_once("://") {
        return Err(format!(
            "has the scheme `{}://`: an endpoint reached by a tunnel, `https://` among them, is written `<host>:<port>`, and takes CONNECT; only `http://<host>:<port>`, on this machine, is marked plain",
            scheme
        ));
    }
    endpoint(text)
}

/// Reads `<host>:<port>` as an endpoint (#195). The error says why it is not one.
pub fn endpoint(text: &str) -> Result<Endpoint, String> {
    let (host, port) = if let Some(rest) = text.strip_prefix('[') {
        let (inside, port) = rest.split_once("]:").ok_or("is not `<host>:<port>`: an IPv6 address is in brackets, followed by `:` and the port")?;
        let ip: Ipv6Addr = inside.parse().map_err(|_| "is not `<host>:<port>`: what is in brackets is not an IPv6 address")?;
        (format!("[{}]", ip), port)
    } else {
        let (host, port) = text.rsplit_once(':').ok_or("is not `<host>:<port>`: it has no port")?;
        if host.contains(':') {
            return Err("is not `<host>:<port>`: an IPv6 address is written in brackets, as `[::1]:8080`".to_string());
        }
        (host_of(host)?, port)
    };
    let digits = !port.is_empty() && port.len() <= 5 && port.bytes().all(|byte| byte.is_ascii_digit()) && !port.starts_with('0');
    let port: u16 = match port.parse() {
        Ok(port) if digits && port > 0 => port,
        _ => return Err("is not `<host>:<port>`: the port is a number from 1 to 65535, with no sign and no leading zero".to_string()),
    };
    Ok(Endpoint { host, port, plain: false })
}

/// A name or an IPv4 address, as it is kept.
fn host_of(host: &str) -> Result<String, String> {
    if host.is_empty() {
        return Err("is not `<host>:<port>`: it has no host".to_string());
    }
    if host.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.') {
        return host
            .parse::<Ipv4Addr>()
            .map(|ip| ip.to_string())
            .map_err(|_| "is not `<host>:<port>`: the host is neither a name nor an IPv4 address".to_string());
    }
    let label = |part: &str| {
        !part.is_empty()
            && part.len() <= 63
            && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !part.starts_with('-')
            && !part.ends_with('-')
    };
    if host.len() > 253 || !host.split('.').all(label) {
        return Err(
            "is not `<host>:<port>`: a name is labels of letters, digits and `-`, 1 to 63 each, split by `.`, none starting or ending with `-`, 253 at most, with no scheme, path, user or wildcard"
                .to_string(),
        );
    }
    Ok(host.to_ascii_lowercase())
}

/// What the sandbox is given for a runner with `reach`: the folder of the proxy's socket, and the program of `wb` that
/// is the forwarder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Net {
    pub folder: PathBuf,
    pub exe: PathBuf,
}

/// The program a test gives as the forwarder: the test process is not `wb`.
static GIVEN_PROGRAM: OnceLock<PathBuf> = OnceLock::new();

/// Takes `program` as the forwarder, in place of the program that runs. For a test, which runs `wb start` in its own
/// process; `wb` itself never calls it.
pub fn use_forwarder(program: &Path) {
    let _ = GIVEN_PROGRAM.set(program.to_path_buf());
}

/// The program of `wb` that is bound as the forwarder: the one that runs (`/proc/self/exe`).
pub fn program() -> Result<PathBuf, String> {
    if let Some(given) = GIVEN_PROGRAM.get() {
        return Ok(given.clone());
    }
    let found = std::env::current_exe().map_err(|error| format!("the program of wb could not be found, and it is the way out of the view: {}", error))?;
    if !found.is_file() {
        return Err(format!("the program of wb, {}, is not there any more, and it is the way out of the view. Run wb start again", found.display()));
    }
    Ok(found)
}

/// One connection the proxy holds open: the session's side, and the far side once it is there.
struct Open {
    near: UnixStream,
    far: Option<TcpStream>,
}

/// What the threads of a proxy share.
struct Shared {
    reach: Vec<Endpoint>,
    file: Option<PathBuf>,
    refused: AtomicUsize,
    written: Mutex<usize>,
    open: Mutex<HashMap<u64, Open>>,
    count: AtomicUsize,
    next: AtomicU64,
    stopping: AtomicBool,
}

impl Shared {
    /// Writes a refusal: the request line, escaped and cut, and why.
    fn refuse(&self, asked: &str, why: &str) {
        self.refused.fetch_add(1, Ordering::SeqCst);
        let Some(file) = &self.file else { return };
        let mut written = self.written.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *written >= MOST_LINES {
            return;
        }
        let shown: String = crate::issue::shown(asked).chars().take(SHOWN).collect();
        let line = format!("{}: {}\n", shown, crate::issue::shown(why));
        let opened = std::fs::OpenOptions::new().create(true).append(true).open(file);
        if let Ok(mut opened) = opened {
            if opened.write_all(line.as_bytes()).is_ok() {
                *written += 1;
            }
        }
    }

    fn hold(&self, id: u64, open: Open) {
        self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(id, open);
    }

    fn far(&self, id: u64, far: &TcpStream) {
        if let Some(open) = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get_mut(&id) {
            open.far = far.try_clone().ok();
        }
    }

    fn release(&self, id: u64) {
        self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&id);
        self.count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The proxy of a session (#195), run as threads of this process. `stop` ends it; so does the end of `wb`.
pub struct Proxy {
    shared: Arc<Shared>,
    accept: Option<std::thread::JoinHandle<()>>,
    socket: PathBuf,
}

impl Proxy {
    /// Makes `folder`, new and for its owner alone, listens on the socket `proxy` in it, and lets through `reach`. Each
    /// refusal is a line of `refused`, where it is given.
    pub fn start(folder: &Path, reach: &[Endpoint], refused: Option<&Path>) -> Result<Proxy, String> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(folder)
            .map_err(|error| format!("the folder of the proxy {} could not be made: {}", folder.display(), error))?;
        // The path of a socket is short (108 bytes): it is bound through the folder held open.
        let held = std::fs::File::open(folder).map_err(|error| format!("the folder of the proxy {} could not be opened: {}", folder.display(), error))?;
        let listener = UnixListener::bind(format!("/proc/self/fd/{}/{}", held.as_raw_fd(), SOCKET_NAME))
            .map_err(|error| format!("the proxy could not listen in {}: {}", folder.display(), error))?;
        drop(held);
        listener.set_nonblocking(true).map_err(|error| format!("the proxy could not listen in {}: {}", folder.display(), error))?;
        let shared = Arc::new(Shared {
            reach: reach.to_vec(),
            file: refused.map(Path::to_path_buf),
            refused: AtomicUsize::new(0),
            written: Mutex::new(0),
            open: Mutex::new(HashMap::new()),
            count: AtomicUsize::new(0),
            next: AtomicU64::new(0),
            stopping: AtomicBool::new(false),
        });
        let inner = shared.clone();
        let accept = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || accepting(listener, inner))
            .map_err(|error| format!("the proxy could not be started: {}", error))?;
        Ok(Proxy { shared, accept: Some(accept), socket: folder.join(SOCKET_NAME) })
    }

    /// Ends the proxy: it takes no more connections, shuts each open one down both ways, waits two seconds at most for
    /// them to end, and removes its socket. Gives how many connections it refused.
    pub fn stop(mut self) -> usize {
        self.end();
        self.shared.refused.load(Ordering::SeqCst)
    }

    fn end(&mut self) {
        self.shared.stopping.store(true, Ordering::SeqCst);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
        for open in self.shared.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).values() {
            let _ = open.near.shutdown(Shutdown::Both);
            if let Some(far) = &open.far {
                let _ = far.shutdown(Shutdown::Both);
            }
        }
        let end = Instant::now() + STOP_WITHIN;
        while self.shared.count.load(Ordering::SeqCst) > 0 && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = std::fs::remove_file(&self.socket);
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        if self.accept.is_some() {
            self.end();
        }
    }
}

/// Takes the connections, until the proxy stops.
fn accepting(listener: UnixListener, shared: Arc<Shared>) {
    while !shared.stopping.load(Ordering::SeqCst) {
        let near = match listener.accept() {
            Ok((near, _)) => near,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
        };
        let _ = near.set_nonblocking(false);
        if shared.count.load(Ordering::SeqCst) >= MOST_OPEN {
            let why = format!("{} connections are open already, the most at once", MOST_OPEN);
            shared.refuse("(not read)", &why);
            answer(&near, "503 Service Unavailable", &why);
            continue;
        }
        let Ok(held) = near.try_clone() else { continue };
        let id = shared.next.fetch_add(1, Ordering::SeqCst);
        shared.count.fetch_add(1, Ordering::SeqCst);
        shared.hold(id, Open { near: held, far: None });
        let inner = shared.clone();
        let spawned = std::thread::Builder::new().stack_size(STACK).spawn(move || {
            serve(near, id, &inner);
            inner.release(id);
        });
        if spawned.is_err() {
            shared.release(id);
        }
    }
}

/// The answer to a request not let through: the status, and one line that says why.
fn answer(near: &UnixStream, status: &str, why: &str) {
    let body = format!("wb: {}\n", why);
    let text = format!("HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, body.len(), body);
    let mut near = near;
    let _ = near.write_all(text.as_bytes());
    let _ = near.shutdown(Shutdown::Both);
}

/// The head of a request, and what came after it, or why it was not read.
fn read_head(near: &mut UnixStream) -> Result<(Vec<u8>, Vec<u8>), (Vec<u8>, &'static str)> {
    let end = Instant::now() + READ_WITHIN;
    let mut got: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 2048];
    loop {
        if let Some(at) = got.windows(4).position(|four| four == b"\r\n\r\n") {
            if at + 4 > HEAD_CAP {
                return Err((got, "the request's head is over 8 KiB"));
            }
            let rest = got.split_off(at + 4);
            return Ok((got, rest));
        }
        if got.len() > HEAD_CAP {
            return Err((got, "the request's head is over 8 KiB"));
        }
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() || near.set_read_timeout(Some(left)).is_err() {
            return Err((got, "the request was not read whole within 10 seconds"));
        }
        match near.read(&mut buffer) {
            Ok(0) => return Err((got, "the request ended before its head did")),
            Ok(count) => got.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                return Err((got, "the request was not read whole within 10 seconds"))
            }
            Err(_) => return Err((got, "the request ended before its head did")),
        }
    }
}

/// The first line of what was read, as text.
fn first_line(head: &[u8]) -> String {
    let line = head.split(|byte| *byte == b'\n').next().unwrap_or_default();
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    String::from_utf8_lossy(&line[..line.len().min(SHOWN * 4)]).to_string()
}

/// Serves one connection of the session: reads its request, and lets it through or refuses it.
fn serve(mut near: UnixStream, id: u64, shared: &Shared) {
    let (head, rest) = match read_head(&mut near) {
        Ok(read) => read,
        Err((got, _)) if got.is_empty() || shared.stopping.load(Ordering::SeqCst) => return,
        Err((got, why)) => {
            shared.refuse(&first_line(&got), why);
            answer(&near, "403 Forbidden", why);
            return;
        }
    };
    let line = first_line(&head);
    // The words of the whole request line: what is shown is cut, and a long path is not.
    let whole = String::from_utf8_lossy(head.split(|byte| *byte == b'\n').next().unwrap_or_default()).trim_end_matches('\r').to_string();
    let words: Vec<&str> = whole.split(' ').collect();
    let (method, target) = match words.as_slice() {
        [method, target, version] if !method.is_empty() && !target.is_empty() && (*version == "HTTP/1.1" || *version == "HTTP/1.0") => (*method, *target),
        _ => {
            let why = "the request is not an HTTP request";
            shared.refuse(&line, why);
            answer(&near, "403 Forbidden", why);
            return;
        }
    };
    // Plain HTTP (#336): a request in absolute form to a plain endpoint, sent on in origin form.
    let (wanted, sent) = if method == "CONNECT" {
        let found = endpoint(target).ok().and_then(|asked| shared.reach.iter().find(|one| one.same_place(&asked)).cloned());
        match found {
            Some(found) => (found, None),
            None => {
                let why = "it is not in `reach` of the runner, and a session in a view reaches nothing else";
                shared.refuse(&line, why);
                answer(&near, "403 Forbidden", &format!("{} {}", target, why.trim_start_matches("it ")));
                return;
            }
        }
    } else {
        match plain_request(&head, &shared.reach) {
            Ok((found, sent)) => (found, Some(sent)),
            Err(why) => {
                // Each says what is let through, and then why this is not.
                let why = if why == NOT_ABSOLUTE { why } else { format!("{}: {}", NOT_ABSOLUTE, why) };
                shared.refuse(&line, &why);
                answer(&near, "403 Forbidden", &why);
                return;
            }
        }
    };
    let far = match connect(&wanted) {
        Ok(far) => far,
        Err(error) => {
            answer(&near, "502 Bad Gateway", &format!("{} could not be reached: {}", wanted, error));
            return;
        }
    };
    shared.far(id, &far);
    if shared.stopping.load(Ordering::SeqCst) {
        let _ = far.shutdown(Shutdown::Both);
        return;
    }
    let _ = near.set_read_timeout(None);
    let mut far = far;
    match sent {
        // A tunnel: what the session sends next is the far side's, unread.
        None => {
            if near.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").is_err() {
                return;
            }
        }
        // Plain HTTP: the head in origin form, and the far side answers.
        Some(sent) => {
            if far.write_all(&sent).is_err() {
                return;
            }
        }
    }
    if !rest.is_empty() && far.write_all(&rest).is_err() {
        return;
    }
    let _ = both_ways(near, far);
}

/// Why plain HTTP is not let through, where the request is not in absolute form.
const NOT_ABSOLUTE: &str = "only CONNECT is let through, or plain HTTP in absolute form, `<method> http://<host>:<port>/<path>`, to a plain endpoint of `reach` of the runner (`http://<host>:<port>`, on this machine)";

/// The form plain HTTP is let through in.
const ABSOLUTE: &str = "plain HTTP is let through as `<method> http://<host>:<port>/<path>`";

/// A request other than `CONNECT` (#336): the plain endpoint of `reach` it is for, and its head in origin form, or why
/// it is refused. `head` is the whole head, read under the limits, up to and with its empty line.
fn plain_request(head: &[u8], reach: &[Endpoint]) -> Result<(Endpoint, Vec<u8>), String> {
    let not_http = || "the request is not an HTTP request".to_string();
    let end = head.windows(2).position(|two| two == b"\r\n").ok_or_else(not_http)?;
    let (first, others) = (&head[..end], &head[end + 2..]);
    let first = std::str::from_utf8(first).ok().filter(|text| text.is_ascii()).ok_or("the request line is not in ASCII")?;
    let [method, target, version] = first.split(' ').collect::<Vec<&str>>()[..] else {
        return Err(not_http());
    };
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(not_http());
    }
    if method.is_empty() || method.len() > 20 || !method.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("the request's method is not a word of capital letters".to_string());
    }
    let after = target.strip_prefix(PLAIN).ok_or(NOT_ABSOLUTE)?;
    let slash = after.find('/').ok_or_else(|| format!("the target has no path: {}", ABSOLUTE))?;
    let (authority, path) = after.split_at(slash);
    if authority.contains('@') {
        return Err(format!("the target names a user: {}", ABSOLUTE));
    }
    if path.bytes().any(|byte| byte <= b' ' || byte == 0x7f || byte == b'#') {
        return Err("the target's path has a fragment, white space or a control character".to_string());
    }
    let asked = endpoint(authority).map_err(|why| format!("the target {}", why.replacen("is not `<host>:<port>`", "is not `http://<host>:<port>/<path>`", 1)))?;
    let found = reach
        .iter()
        .find(|one| one.same_place(&asked))
        .ok_or_else(|| format!("{} is not in `reach` of the runner, and a session in a view reaches nothing else", asked))?;
    if !found.plain {
        return Err(format!("{} takes CONNECT alone: it is not marked plain (`http://`) in `reach` of the runner, so plain HTTP is not let through to it", found));
    }
    if !PLAIN_HOSTS.contains(&found.host.as_str()) {
        return Err("plain HTTP is let through to this machine alone".to_string());
    }
    // The lines of the head after the request line, each ending with CRLF, then the empty line.
    let mut lines = others.strip_suffix(b"\r\n").ok_or_else(not_http)?;
    let mut hosts: Vec<&[u8]> = Vec::new();
    while !lines.is_empty() {
        let end = lines.windows(2).position(|two| two == b"\r\n").ok_or("a line of the head does not end with CRLF")?;
        let line = &lines[..end];
        lines = &lines[end + 2..];
        if line.contains(&b'\r') || line.contains(&b'\n') {
            return Err("a line of the head has a CR or a LF alone".to_string());
        }
        if line.first().is_some_and(|byte| *byte == b' ' || *byte == b'\t') {
            return Err("a line of the head is folded: it starts with white space".to_string());
        }
        let colon = line.iter().position(|byte| *byte == b':').ok_or("a line of the head has no colon")?;
        let name = &line[..colon];
        if name.is_empty() || name.iter().any(|byte| *byte <= b' ' || *byte == 0x7f) {
            return Err("a line of the head has white space or nothing before its colon".to_string());
        }
        if name.eq_ignore_ascii_case(b"host") {
            hosts.push(&line[colon + 1..]);
        }
    }
    let host = match hosts[..] {
        [] => return Err("the request has no `Host:` header".to_string()),
        [one] => String::from_utf8_lossy(one).to_string(),
        _ => return Err("the request has more than one `Host:` header".to_string()),
    };
    let host = host.trim_matches(|c| c == ' ' || c == '\t');
    match endpoint(host) {
        Ok(named) if named.same_place(&asked) => {}
        _ => return Err(format!("the `Host:` header, {}, is not the target's host and port, {}", crate::issue::shown(host), asked)),
    }
    let mut sent = format!("{} {} {}\r\n", method, path, version).into_bytes();
    sent.extend_from_slice(others);
    Ok((found.clone(), sent))
}

/// A connection to an endpoint, resolved here, outside the sandbox, within `CONNECT_WITHIN` for each address. A plain
/// endpoint is connected to only at a loopback address (#336).
fn connect(wanted: &Endpoint) -> std::io::Result<TcpStream> {
    let none = if wanted.plain { "the name leads to no loopback address" } else { "the name leads to no address" };
    let mut last = std::io::Error::new(std::io::ErrorKind::NotFound, none);
    for address in (wanted.bare(), wanted.port).to_socket_addrs()? {
        if wanted.plain && !address.ip().is_loopback() {
            continue;
        }
        match TcpStream::connect_timeout(&address, CONNECT_WITHIN) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = error,
        }
    }
    Err(last)
}

/// Something both ways can be passed through, and shut down.
trait Pipe: Read + Write + Send + 'static {
    fn twin(&self) -> std::io::Result<Self>
    where
        Self: Sized;
    fn close(&self);
}

impl Pipe for UnixStream {
    fn twin(&self) -> std::io::Result<Self> {
        self.try_clone()
    }
    fn close(&self) {
        let _ = self.shutdown(Shutdown::Both);
    }
}

impl Pipe for TcpStream {
    fn twin(&self) -> std::io::Result<Self> {
        self.try_clone()
    }
    fn close(&self) {
        let _ = self.shutdown(Shutdown::Both);
    }
}

/// Passes the bytes of `one` to `other` and back, unread, until either side ends; then both are shut down both ways.
fn both_ways<A: Pipe, B: Pipe>(one: A, other: B) -> std::io::Result<()> {
    let (one_back, other_back) = (one.twin()?, other.twin()?);
    let (one_end, other_end) = (one.twin()?, other.twin()?);
    let back = std::thread::Builder::new().stack_size(STACK).spawn(move || {
        let (mut from, mut to) = (other_back, one_back);
        let _ = std::io::copy(&mut from, &mut to);
        from.close();
        to.close();
    })?;
    let (mut from, mut to) = (one, other);
    let _ = std::io::copy(&mut from, &mut to);
    one_end.close();
    other_end.close();
    let _ = back.join();
    Ok(())
}

/// `wb` as the forwarder, with the words after its name: `-- <program> <words>…` listens on `127.0.0.1:PORT`, starts
/// the copy that passes the connections on, and runs the program in its own place; `--listening` is that copy, with
/// the listening socket as its standard input. Gives the status where the program could not be run.
pub fn forward(args: &[OsString]) -> i32 {
    match args {
        [word] if word == LISTENING => {
            listening();
            0
        }
        [dashes, program, words @ ..] if dashes == "--" => run_through(program, words),
        _ => {
            let _ = writeln!(std::io::stderr(), "wb: {} is the way out of a view, and is run by wb start alone", FORWARDER);
            2
        }
    }
}

/// Listens, starts the copy that passes the connections on, and runs `program` in this process's place.
fn run_through(program: &OsString, words: &[OsString]) -> i32 {
    use std::os::fd::OwnedFd;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    let mut err = std::io::stderr();
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(listener) => listener,
        Err(error) => {
            let _ = writeln!(err, "wb: the session is not started: the way out of the view could not listen on 127.0.0.1:{}: {}", PORT, error);
            // not a refusal: the status of the session in the sandbox, which `wb start` says as a session's end.
            return 1;
        }
    };
    let socket: OwnedFd = listener.into();
    let copy = Command::new(FORWARDER).arg(LISTENING).stdin(Stdio::from(socket)).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    if let Err(error) = copy {
        let _ = writeln!(err, "wb: the session is not started: the way out of the view could not be started: {}", error);
        // not a refusal: the status of the session in the sandbox, which `wb start` says as a session's end.
        return 1;
    }
    let error = Command::new(program).args(words).exec();
    let _ = writeln!(err, "wb: the session could not be started: {}: {}", program.to_string_lossy(), error);
    // not a refusal: the status of the session in the sandbox, which `wb start` says as a session's end.
    1
}

/// Passes each connection taken on standard input to the proxy's socket, until the sandbox ends.
fn listening() {
    use std::os::fd::FromRawFd;
    // Standard input is the listening socket that `run_through` handed on.
    let listener = unsafe { TcpListener::from_raw_fd(0) };
    loop {
        match listener.accept() {
            Ok((near, _)) => {
                let _ = std::thread::Builder::new().stack_size(STACK).spawn(move || {
                    if let Ok(far) = UnixStream::connect(SOCKET) {
                        let _ = both_ways(near, far);
                    }
                });
            }
            // Standard input that is not a listening socket: there is nothing to pass on.
            Err(error) if matches!(error.raw_os_error(), Some(ENOTSOCK) | Some(EINVAL) | Some(EBADF)) => return,
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

const EBADF: i32 = 9;
const EINVAL: i32 = 22;
const ENOTSOCK: i32 = 88;
