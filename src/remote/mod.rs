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
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

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

/// The same pairing page under an origin that is already named: the
/// public URL a proxy serves, or the tailnet's HTTPS name. The code rides
/// raw, as in [`pairing_url`].
pub fn pairing_url_at(base: &str, code: &str) -> String {
    format!("{base}/pair?c={code}")
}

/// The origin a pairing URL should use, and how it was chosen. The note
/// is the honest half: a fallback says it is plain HTTP, and a tailnet
/// name that could not be verified says what to point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairOrigin {
    pub base: String,
    pub kind: PairOriginKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairOriginKind {
    /// `remote.public_url`, the owner's explicit origin.
    PublicUrl,
    /// A tailnet HTTPS name, verified to reach the remote face.
    Tailnet,
    /// The listener's own address, spoken over plain HTTP.
    Lan,
}

impl PairOriginKind {
    pub fn label(self) -> &'static str {
        match self {
            PairOriginKind::PublicUrl => "the public URL in your settings",
            PairOriginKind::Tailnet => "your tailnet name",
            PairOriginKind::Lan => "this machine's address",
        }
    }

    /// The wire name the card receives.
    pub fn key(self) -> &'static str {
        match self {
            PairOriginKind::PublicUrl => "public_url",
            PairOriginKind::Tailnet => "tailnet",
            PairOriginKind::Lan => "lan",
        }
    }
}

/// The origin a phone should open, in order of how much it can be
/// trusted: an explicit public URL, a verified tailnet HTTPS name, the
/// listener's address.
///
/// The tailnet name is verified rather than assumed. The remote face
/// answers 401 to `/api/state` without a device cookie; the loopback face
/// answers 200. A proxy pointed at the loopback face is therefore not a
/// pairing origin — `/pair` does not exist there — and the card says so
/// instead of putting a URL that 404s into a QR.
pub fn pairing_origin(home: &Path, address: IpAddr, port: u16) -> PairOrigin {
    if let Some(base) = configured_pair_url(home) {
        return PairOrigin {
            base,
            kind: PairOriginKind::PublicUrl,
            note: None,
        };
    }
    if let Some(dns) = tailscale_dns_name() {
        let base = format!("https://{dns}");
        return match probe_origin(&base) {
            OriginProbe::Remote => PairOrigin {
                base,
                kind: PairOriginKind::Tailnet,
                note: None,
            },
            OriginProbe::Loopback => lan_origin(
                address,
                port,
                Some(
                    "Tailscale is running, but its HTTPS origin serves the loopback face, not \
                     the paired one — /pair is not there. Point `tailscale serve` at the \
                     remote face (the address printed at launch), or set `remote.public_url`."
                        .to_string(),
                ),
            ),
            OriginProbe::Unreachable(reason) => lan_origin(
                address,
                port,
                Some(format!(
                    "Tailscale is running ({dns}), but {base} did not answer as the remote \
                     face ({reason}). Point `tailscale serve` at the remote face, or set \
                     `remote.public_url`."
                )),
            ),
        };
    }
    lan_origin(address, port, None)
}

fn lan_origin(address: IpAddr, port: u16, note: Option<String>) -> PairOrigin {
    let authority = match address {
        IpAddr::V6(address) => format!("[{address}]:{port}"),
        IpAddr::V4(address) => format!("{address}:{port}"),
    };
    PairOrigin {
        base: format!("http://{authority}"),
        kind: PairOriginKind::Lan,
        note: Some(note.unwrap_or_else(|| {
            "Plain HTTP: nearby devices only. iOS refuses cleartext to a tailnet address — \
             use `tailscale serve` or set `remote.public_url` for HTTPS."
                .to_string()
        })),
    }
}

/// `remote.public_url` from the home's config, when it is an http(s)
/// origin. A value that is not one is ignored rather than pasted into a
/// QR; the card falls through to the tailnet or the LAN address.
pub fn configured_pair_url(home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(home.join("config.json")).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    let url = parsed
        .get("remote")?
        .get("public_url")?
        .as_str()?
        .trim()
        .trim_end_matches('/')
        .to_string();
    (url.starts_with("http://") || url.starts_with("https://")).then_some(url)
}

enum OriginProbe {
    Remote,
    Loopback,
    Unreachable(String),
}

/// Ask the origin what it is. A remote face without a device cookie is a
/// 401; a loopback face is a 200; anything else did not answer as either.
fn probe_origin(base: &str) -> OriginProbe {
    // The bound is the whole probe, not just the socket: a resolver waiting
    // on a tailnet name it cannot reach does not heed ureq's timeout, and a
    // pairing card that waits on it is a card that never opens. The worker
    // may linger until the resolver gives up; the card does not.
    let (sender, receiver) = mpsc::channel();
    let base = base.to_string();
    std::thread::spawn(move || {
        let _ = sender.send(probe_origin_blocking(&base));
    });
    receiver
        .recv_timeout(Duration::from_secs(4))
        .unwrap_or_else(|_| OriginProbe::Unreachable("no answer within four seconds".to_string()))
}

fn probe_origin_blocking(base: &str) -> OriginProbe {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(3))
        .build();
    match agent.get(&format!("{base}/api/state")).call() {
        Ok(_) => OriginProbe::Loopback,
        Err(ureq::Error::Status(401, _)) => OriginProbe::Remote,
        Err(ureq::Error::Status(status, _)) => OriginProbe::Unreachable(format!("HTTP {status}")),
        Err(ureq::Error::Transport(error)) => OriginProbe::Unreachable(error.to_string()),
    }
}

/// The tailnet's own HTTPS name, when the daemon is running. Read-only:
/// `tailscale status --json` never changes the mesh. The read is bounded,
/// because a daemon that is not answering must not hold the pairing card.
fn tailscale_dns_name() -> Option<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let output = Command::new("tailscale")
            .args(["status", "--json"])
            .output()
            .ok();
        let _ = sender.send(output);
    });
    let output = receiver.recv_timeout(Duration::from_secs(2)).ok()??;
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    if parsed
        .get("BackendState")
        .and_then(serde_json::Value::as_str)
        != Some("Running")
    {
        return None;
    }
    let name = parsed
        .get("Self")?
        .get("DNSName")?
        .as_str()?
        .trim_end_matches('.')
        .to_string();
    (!name.is_empty()).then_some(name)
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
    socket.connect((Ipv4Addr::new(198, 51, 100, 1), 80)).ok()?;
    match socket.local_addr() {
        Ok(SocketAddr::V4(address)) if !address.ip().is_loopback() => {
            Some(IpAddr::V4(*address.ip()))
        }
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
    fn the_public_url_is_used_only_when_it_is_an_http_origin() {
        let home = std::env::temp_dir().join(format!(
            "fleet-pair-origin-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(
            home.join("config.json"),
            r#"{"remote":{"public_url":"https://phone.example/"}}"#,
        )
        .unwrap();
        assert_eq!(
            configured_pair_url(&home).as_deref(),
            Some("https://phone.example")
        );
        std::fs::write(
            home.join("config.json"),
            r#"{"remote":{"public_url":"file:///etc/passwd"}}"#,
        )
        .unwrap();
        assert!(configured_pair_url(&home).is_none());
        std::fs::write(home.join("config.json"), "{}").unwrap();
        assert!(configured_pair_url(&home).is_none());
        std::fs::remove_dir_all(&home).ok();
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
