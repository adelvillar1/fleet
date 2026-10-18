//! The remote face: a paired device reaching Fleet from another machine.
//!
//! Two pieces live here. [`pairing`] is the credential — a one-time code
//! that exchanges for a device token, remembered as a hash. [`listener`]
//! is the second HTTP surface: a listener that is *not* the loopback
//! face, bound to an address another machine can reach, gated by device
//! tokens, and **default-deny** on its routes — a paired phone may drive
//! a session, and nothing else, until a route is deliberately added.
//!
//! The loopback face is untouched by any of this: it keeps its own
//! listener, its own per-launch token, and its refusal of a non-loopback
//! `--host` by name. This module adds a face beside it, not a mode the
//! existing one enters.

pub mod listener;
pub mod pairing;
pub mod qr;
pub mod relay;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

/// What the owner asked the remote face to do.
#[derive(Debug, Clone, Default)]
pub struct RemoteOptions {
    /// Serve the remote face at all? The default is not to: with no
    /// paired devices and no `--remote`, this face does not exist.
    pub enabled: bool,
    /// An address the owner named explicitly, overriding the detected
    /// one. A hostname is resolved; anything unresolvable is an error
    /// named at the prompt rather than at bind time.
    pub host: Option<String>,
}

/// The address the remote face binds, and how it was chosen — which the
/// printed pairing block names, because "the address we detected" is a
/// thing the owner should be able to check.
#[derive(Debug, Clone)]
pub struct RemoteAddress {
    pub address: IpAddr,
    pub kind: AddressKind,
}

/// Where an address came from. `Detected` is the machine's primary
/// routable address; `Named` is one the owner typed; `Tailnet` is the
/// CGNAT range a mesh network hands out, which keeps working away from
/// home the way a LAN address does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressKind {
    Detected,
    Named,
    Tailnet,
}

impl AddressKind {
    /// How the printed pairing block names where the address came from.
    /// The owner should be able to check the choice at a glance.
    pub fn label(self) -> &'static str {
        match self {
            AddressKind::Detected => "this machine's detected address",
            AddressKind::Named => "the address you named",
            AddressKind::Tailnet => "your tailnet address",
        }
    }
}

/// The URL a pairing QR encodes and the terminal prints: the pairing
/// page with the code already in the query, so a phone that scans it
/// lands on the form with the code filled in and one tap from paired.
/// The code rides raw — no dashes — because a query parameter is not a
/// human reading a screen.
pub fn pairing_url(address: IpAddr, port: u16, code: &str) -> String {
    let authority = match address {
        IpAddr::V6(address) => format!("[{address}]:{port}"),
        IpAddr::V4(address) => format!("{address}:{port}"),
    };
    format!("http://{authority}/pair?c={code}")
}

/// Resolve the remote face's address. The owner's `--remote-host` wins
/// when it is given and resolves; otherwise the machine's primary
/// routable address is detected, and a failure to detect one is an
/// error that names the flag — a face with nowhere to bind is a usage
/// error, not a silent loopback fallback.
pub fn resolve_address(options: &RemoteOptions) -> Result<RemoteAddress, String> {
    if let Some(named) = &options.host {
        let address = resolve_named(named)?;
        let kind = if is_tailnet(address) {
            AddressKind::Tailnet
        } else {
            AddressKind::Named
        };
        return Ok(RemoteAddress { address, kind });
    }
    match detect_primary() {
        Some(address) => Ok(RemoteAddress {
            address,
            kind: if is_tailnet(address) {
                AddressKind::Tailnet
            } else {
                AddressKind::Detected
            },
        }),
        None => Err(
            "fleet could not detect this machine's address; pass --remote-host <address>"
                .to_string(),
        ),
    }
}

/// Resolve a named host. A literal IP is taken as-is; a name is resolved
/// through the operating system, first answer only — a name with several
/// addresses is a machine with several faces, and the owner should say
/// which one rather than let the resolver's order decide.
fn resolve_named(host: &str) -> Result<IpAddr, String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("fleet --remote-host needs an address".to_string());
    }
    if let Ok(address) = trimmed.parse::<IpAddr>() {
        return Ok(address);
    }
    let mut addrs = (trimmed, 0u16)
        .to_socket_addrs()
        .map_err(|error| format!("fleet cannot resolve --remote-host {trimmed}: {error}"))?;
    addrs
        .next()
        .map(|socket| socket.ip())
        .ok_or_else(|| format!("fleet cannot resolve --remote-host {trimmed}"))
}

/// This machine's primary routable IPv4 address, found the way a program
/// without an interface table finds it: open a UDP socket toward a
/// public address and ask the operating system which local address it
/// would use. Nothing is sent — a UDP `connect` is a routing decision,
/// not a handshake — so this works with no network permission beyond
/// the ability to route, and returns the address the default route
/// would carry.
fn detect_primary() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    // A public address that is never contacted: 198.51.100.1 is the
    // TEST-NET-2 block RFC 5737 set aside for documentation.
    socket
        .connect((Ipv4Addr::new(198, 51, 100, 1), 80))
        .ok()?;
    match socket.local_addr() {
        Ok(SocketAddr::V4(address)) if !address.ip().is_loopback() => Some(IpAddr::V4(*address.ip())),
        _ => None,
    }
}

/// True for the CGNAT range a tailnet hands out (RFC 6598's
/// 100.64.0.0/10). Unlike a LAN address it keeps working away from home,
/// which is why the pairing block names it differently.
pub fn is_tailnet(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let first = address.octets()[0];
            let second = address.octets()[1];
            first == 100 && (64..=127).contains(&second)
        }
        IpAddr::V6(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_literal_ip_is_taken_as_it_is() {
        let options = RemoteOptions {
            enabled: true,
            host: Some("192.168.1.50".to_string()),
        };
        let resolved = resolve_address(&options).unwrap();
        assert_eq!(
            resolved.address,
            "192.168.1.50".parse::<IpAddr>().unwrap(),
            "{resolved:?}"
        );
        assert_eq!(resolved.kind, AddressKind::Named);
    }

    #[test]
    fn a_named_tailnet_address_is_recognised_as_one() {
        let options = RemoteOptions {
            enabled: true,
            host: Some("100.101.102.103".to_string()),
        };
        let resolved = resolve_address(&options).unwrap();
        assert_eq!(resolved.kind, AddressKind::Tailnet);
    }

    #[test]
    fn a_named_host_that_resolves_nowhere_is_an_error_naming_the_flag() {
        let options = RemoteOptions {
            enabled: true,
            host: Some("no-such-host.invalid".to_string()),
        };
        let error = resolve_address(&options).unwrap_err();
        assert!(error.contains("--remote-host"), "{error}");
        assert!(error.contains("no-such-host.invalid"), "{error}");
    }

    #[test]
    fn an_empty_named_host_is_refused_before_a_socket_exists() {
        let options = RemoteOptions {
            enabled: true,
            host: Some("   ".to_string()),
        };
        assert!(resolve_address(&options).is_err());
    }

    #[test]
    fn localhost_resolves_to_a_name_but_it_is_not_a_tailnet() {
        let options = RemoteOptions {
            enabled: true,
            host: Some("localhost".to_string()),
        };
        let resolved = resolve_address(&options).unwrap();
        assert!(resolved.address.is_loopback(), "{resolved:?}");
        assert_eq!(resolved.kind, AddressKind::Named);
    }

    #[test]
    fn the_tailnet_range_is_exactly_its_bounds() {
        assert!(is_tailnet("100.64.0.0".parse().unwrap()));
        assert!(is_tailnet("100.127.255.255".parse().unwrap()));
        assert!(!is_tailnet("100.63.255.255".parse().unwrap()));
        assert!(!is_tailnet("100.128.0.0".parse().unwrap()));
        assert!(!is_tailnet("192.168.1.5".parse().unwrap()));
    }

    #[test]
    fn detection_never_offers_loopback() {
        // Whatever this machine's routing says, a loopback answer is not
        // a face another device can reach, so the seam refuses it and
        // the caller names the flag instead.
        if let Some(address) = detect_primary() {
            assert!(!address.is_loopback(), "{address}");
        }
    }
}
