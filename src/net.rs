//! TCP reachability probing for the doctor.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// True when some address of `host:port` accepts a TCP connection within the
/// probe window. Resolving and connecting stay separate so a multi-address
/// host falls through to the reachable one.
pub fn listening(host: &str, port: u16, timeout: Duration) -> bool {
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_listener_is_listening() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(listening("127.0.0.1", port, Duration::from_millis(500)));
    }

    #[test]
    fn a_dead_port_is_not_listening() {
        assert!(!listening("127.0.0.1", 1, Duration::from_millis(200)));
    }
}
