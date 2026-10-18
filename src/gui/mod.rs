//! The GUI: `fleet gui` boots a browser face over the same agent core
//! the TUI and the headless runs use.
//!
//! The shape is decided by the crate contract — one static Rust binary,
//! no Node runtime, no checkout dependency — so the surface is a
//! hand-rolled HTTP/1.1 server on tokio's `TcpListener` ([`server`])
//! with the page, its stylesheet and its script embedded in the binary
//! by `include_str!`. There is no framework, no build step, and no
//! asset directory to ship beside the binary.
//!
//! ## The security shape
//!
//! - **Loopback only.** The listener binds 127.0.0.1 and a `--host`
//!   that is not loopback is refused by name before a socket exists.
//!   The surface is the owner's machine and nothing else.
//! - **The address alone opens the face.** The listener is loopback
//!   only, and the server checks that a request's Host and Origin name
//!   this machine — which is what keeps a rebinding page or a
//!   cross-site form from driving the agent through the local port. A
//!   browser on this machine fails neither check, so the owner browses
//!   the address and is in; a port scan sees 403s and learns nothing.
//!   Authentication is a multi-user future's need, and returns when
//!   that future arrives.
//! - **No credentials in any payload.** The page, the assets, and the
//!   stream carry names and presence, never values: the same masking
//!   rule doctor holds.
//! - **Every write goes through the one writer.** `settings.rs` owns
//!   the configuration files; this surface reads state and posts
//!   actions, and an action is a settings write or an agent run, never
//!   a second path to a file.
//!
//! The TUI is untouched by any of this: it is the other face of the
//! same core, and this module adds a third rather than replacing one.

pub mod bridge;
pub mod related;
pub mod server;
pub mod themes;
pub mod usage;

use std::process::Command;
use std::sync::Arc;

use crate::remote::listener;

/// What `fleet gui` is told to do.
pub struct GuiOptions {
    /// The address to bind. Validated loopback before a socket exists.
    pub host: String,
    /// The port to bind; zero asks the operating system for a free one.
    pub port: u16,
    /// Open the browser anyway? `true` means do not: the address is
    /// printed, and the owner opens the page themselves.
    pub no_open: bool,
    /// The paired-device face beside this one. Off by default, and off
    /// means no second listener and no address beyond loopback.
    pub remote: crate::remote::RemoteOptions,
    /// The channel service beside the faces. Off by default, and off
    /// means no channel listener and no port.
    pub channels: bool,
    /// Dial out to this relay so a phone on another network reaches the
    /// remote face through it. Off by default.
    pub relay: Option<String>,
}

/// Where the face listens unless told otherwise.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// The port the face listens on unless told otherwise. 3080 is away
/// from the harness's 8300 and the fleet router's own space, so a
/// running fleet never has to guess which surface a port belongs to.
pub const DEFAULT_PORT: u16 = 3080;

/// What every GUI run's prompt says about the surface, before the
/// address. One constant so the seam's test and the docs read the same
/// sentence.
const GUI_PROMPT: &str = "\
The owner is talking to you through Fleet's browser face, live for this run. \
It streams your progress as it happens — every turn, tool call, and result — \
and its composer is where the owner's messages arrive. Nothing about the \
surface changes what you do: the run is the same one the terminal runs, and \
the owner can watch it and steer it from there.";

/// The GUI's standing line with the address the owner opened. The line
/// names the surface and the address and never the capability that
/// reaches them — the browser holds that, and a prompt is the wrong
/// place for it.
pub fn prompt_line(base_url: &str) -> String {
    format!("{GUI_PROMPT} It is open at {base_url}.")
}

/// Boots the face: bind, announce, open, serve until the owner stops
/// the process. The URL is the machine's own address, and the browser
/// already carries everything it needs to be let in
/// once, and it is printed to the terminal only — never logged, never
/// written to a file, never carried in a payload.
///
/// The home and the session are resolved here exactly the way `tui::run`
/// resolves them, so the browser face and the terminal face are the same
/// core over a different seat: one home, one fresh session, one agent
/// thread behind the bridge.
pub async fn serve(options: GuiOptions) -> Result<(), String> {
    let user_home = std::env::var("HOME").ok();
    let home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        user_home.as_deref(),
    )?;
    let session = crate::session::Session::create(&home, crate::api::DEFAULT_MODEL)
        .map_err(|error| format!("could not create the session: {error}"))?;
    let base_url = format!("http://{}:{}", options.host, options.port);
    let bridge = Arc::new(bridge::Bridge::spawn(
        home.clone(),
        crate::api::DEFAULT_MODEL.to_string(),
        session.id().to_string(),
        base_url,
    ));
    let (server, _port) =
        server::Server::bind(&options.host, options.port, Arc::clone(&bridge)).await?;
    let url = server.base_url().to_string();
    println!("fleet gui — the browser face is live");
    println!("  {url}");
    if options.no_open {
        println!("  --no-open: the browser was left alone; open the URL above.");
    } else if let Err(e) = open_browser(&url) {
        // A browser that will not open is a convenience lost, not a
        // failure: the URL is on the screen and the server is live.
        println!("  could not open a browser ({e}); open the URL above.");
    }

    // The remote face, opt-in. The address is resolved before anything is
    // bound and before a window is minted, so a face that cannot come up
    // is a named error rather than a half-up one with a live code nobody
    // was told about. Both faces share one bridge — a phone and the
    // desktop are two clients of one conversation.
    let remote = if options.remote.enabled {
        let address = crate::remote::resolve_address(&options.remote)?;
        let (listener, port) =
            listener::Listener::bind(&address, options.port, home.clone(), Arc::clone(&bridge))
                .await?;
        let (_window, code) = crate::remote::pairing::PairingWindow::mint(&home)
            .map_err(|error| format!("could not mint a pairing code: {error}"))?;
        println!("fleet gui — the remote face is live on {}", address_name(address.address));
        println!(
            "  pair:  http://{}:{port}/pair  (code {})",
            address_name(address.address),
            crate::remote::pairing::display_code(&code)
        );
        println!(
            "  {} · the code lives {} minutes and burns on use · `fleet devices` lists what paired",
            address.kind.label(),
            crate::remote::pairing::WINDOW_TTL_MINUTES
        );
        // The QR. Direct, it is the pairing page with the code in the
        // query, so a scan lands on the form with the code filled. Through
        // a relay, it is the relay's entry point with the device session
        // and its secret — the phone that scans it is the phone that can
        // reach this face through the relay, which is the same shape
        // ZCode's own remote control uses.
        let url = match &options.relay {
            Some(relay) => format!(
                "{}/s/{}?hash={}",
                relay.trim_end_matches('/'),
                listener.device_session(),
                listener.device_secret()
            ),
            None => crate::remote::pairing_url(address.address, port, &code),
        };
        match crate::remote::qr::encode(&url) {
            Ok(matrix) => {
                println!();
                print!("{}", matrix.to_ansi());
            }
            Err(error) => println!("  (no QR: {error}) — type the code instead"),
        }
        Some(listener)
    } else {
        None
    };
    println!("  Ctrl-C stops it.");

    // The channel service, opt-in: a transport plugin carrying work from
    // outside. It runs on the same bridge, so a peer's task is an
    // ordinary run in that peer's own session — the same agent, the same
    // log, the same seat — and nothing here is a second path to the core.
    let _channels = if options.channels {
        let home = home.clone();
        let policy = crate::channels::Policy::new(home.clone());
        let driver = Arc::new(crate::channels::Driver::new(
            Arc::clone(&bridge),
            policy,
        ));
        let webhook = crate::channels::webhook::Webhook::new(
            crate::channels::webhook::DEFAULT_PORT,
            home,
            Arc::clone(&driver),
        );
        // The watcher presents the channel's seats and posts its outcomes,
        // subscribed to the same stream the browser watches.
        let channel: Arc<dyn crate::channels::Channel> = Arc::new(webhook);
        let served = Arc::clone(&channel);
        let delivering = Arc::clone(&driver);
        tokio::spawn(async move {
            let sink: Arc<
                dyn Fn(crate::channels::Inbound) -> Result<String, crate::channels::Refusal>
                    + Send
                    + Sync,
            > = Arc::new(move |inbound| delivering.deliver(&inbound));
            let _ = served.serve(sink).await;
        });
        let watching = Arc::clone(&driver);
        tokio::spawn(async move { watching.watch(channel).await });
        println!(
            "fleet gui — the webhook channel is live on 127.0.0.1:{}",
            crate::channels::webhook::DEFAULT_PORT
        );
        Some(driver)
    } else {
        None
    };

    // The relay: Fleet dials out so a phone on cellular can reach the
    // remote face with nothing installed on the network. The relay is a
    // transport, not a second implementation: the frames it forwards are
    // answered by the remote listener's own router.
    if let Some(relay_url) = &options.relay {
        let Some(listener) = remote.as_ref() else {
            return Err(
                "fleet gui --relay needs --remote: a relay with no remote face has nothing to forward"
                    .to_string(),
            );
        };
        // The device session the pairing window minted is the relay's
        // credential, so a relay can only ever be joined by a device that
        // paired.
        let sid = listener.device_session();
        let hash = listener.device_secret();
        let client = Arc::new(crate::remote::relay::Client::new(
            relay_url,
            sid,
            hash,
        ));
        let handler = Arc::new(listener::RelayHandler::new(
            Arc::clone(&bridge),
            "a relayed phone".to_string(),
        ));
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let dialing = Arc::clone(&client);
        tokio::spawn(async move {
            crate::remote::relay::serve(dialing, handler, shutdown_rx).await
        });
        println!("fleet gui — dialing the relay at {relay_url}");
        // The entry point as text as well as a QR: a phone that cannot
        // scan (a screenshot, a copy-paste) still has a way in, and the
        // owner can see exactly what the code carries.
        println!(
            "  phone: {}/s/{}?hash={}",
            relay_url.trim_end_matches('/'),
            listener.device_session(),
            listener.device_secret()
        );
        // The shutdown handle is held for the process's life: a relay that
        // stops when the face stops is the honest shape.
        let _ = shutdown;
    }

    match remote {
        // Both faces serve until the owner stops the process; either one
        // ending is the process ending.
        Some(listener) => {
            tokio::select! {
                ended = server.serve() => ended?,
                ended = listener.serve() => ended?,
            }
        }
        None => server.serve().await?,
    }
    println!("fleet gui — stopped.");
    Ok(())
}

/// An address as a URL authority: IPv6 bracketed, IPv4 bare.
fn address_name(address: std::net::IpAddr) -> String {
    match address {
        std::net::IpAddr::V6(address) => format!("[{address}]"),
        std::net::IpAddr::V4(address) => address.to_string(),
    }
}

/// Hands the URL to the operating system's opener. macOS has `open`;
/// everything else that matters here has `xdg-open`; a machine with
/// neither gets an error the caller turns into a printed line.
fn open_browser(url: &str) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(url)
        .status()
        .map_err(|e| format!("{program} could not run: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}
