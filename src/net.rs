//! TCP probing for the `status` command and the doctor's web check.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// True when some address of `host:port` accepts a TCP connection within the
/// probe window. Resolving and connecting stay separate so a multi-address
/// host falls through to the reachable one.
pub fn listening(host: &str, port: u16) -> bool {
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok() {
            return true;
        }
    }
    false
}
