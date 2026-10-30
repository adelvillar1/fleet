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
    let base_url = format!("http://{}:{}", options.host, options.port);
    let bridge = Arc::new(bridge::Bridge::spawn(
        home.clone(),
        crate::route::AUTO_MODEL.to_string(),
        None,
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
    // The bot face's relay credentials, captured at bind: the listener
    // moves into its serve task, and the bot's relay dial (below) needs
    // the session and secret that were minted for it alone.
    let mut bot_relay_credentials: Option<(String, String)> = None;
    let remote = if options.remote.enabled {
        let address = crate::remote::resolve_address(&options.remote)?;
        // Two faces, one bridge (the owner's 2026-10-28 correction): the
        // remote console face keeps the work experience on the origin
        // the owner's PWA was installed from, and the bot face — the
        // persona experience — is a second window on its own port.
        let (listener, port) = listener::Listener::bind(
            &address,
            options.port,
            listener::RemoteFace::Console,
            home.clone(),
            Arc::clone(&bridge),
        )
        .await?;
        bridge.attach_remote(address.address, port);
        let bot_address = crate::remote::RemoteAddress {
            address: address.address,
            kind: address.kind,
        };
        let (bot_listener, bot_port) = listener::Listener::bind(
            &bot_address,
            options.remote.bot_port,
            listener::RemoteFace::Bot,
            home.clone(),
            Arc::clone(&bridge),
        )
        .await?;
        // The Robots surface mints a card for this face: its LAN port
        // now, its relay origin when the bot block dials one.
        bridge.attach_bot_face(bot_port);
        bot_relay_credentials = Some((bot_listener.device_session(), bot_listener.device_secret()));
        tokio::spawn(async move {
            if let Err(error) = bot_listener.serve().await {
                eprintln!("fleet remote: the bot face stopped: {error}");
            }
        });
        let bot_url = format!(
            "http://{}:{bot_port}",
            match bot_address.address {
                std::net::IpAddr::V4(ip) => ip.to_string(),
                std::net::IpAddr::V6(ip) => ip.to_string(),
            }
        );
        println!("fleet gui — the bot face is live on the same address, port {bot_port}");
        println!("  bot:   {bot_url}  (the persona experience; pair it the same way)");
        let (_window, code) = crate::remote::pairing::PairingWindow::mint(&home)
            .map_err(|error| format!("could not mint a pairing code: {error}"))?;
        // The explicit public URL is used when the owner has set one; the
        // in-app card does the Tailscale detection on demand, so launch
        // stays fast.
        let public = crate::remote::configured_pair_url(&home);
        let url = match &public {
            Some(base) => crate::remote::pairing_url_at(base, &code),
            None => crate::remote::pairing_url(address.address, port, &code),
        };
        println!(
            "fleet gui — the remote face is live on {}",
            address_name(address.address)
        );
        println!(
            "  pair:  {url}  (code {})",
            crate::remote::pairing::display_code(&code)
        );
        println!(
            "  {} · the code lives {} minutes and burns on use · `fleet devices` lists what paired",
            if public.is_some() {
                "the public URL in your settings"
            } else {
                address.kind.label()
            },
            crate::remote::pairing::WINDOW_TTL_MINUTES
        );
        println!("  the QR icon in the face's topbar finds your Tailscale name");

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

    // The channel service: a transport plugin carrying work from outside.
    // It runs on the same bridge, so a peer's task is an ordinary run in
    // that peer's own session — the same agent, the same log, the same
    // seat — and nothing here is a second path to the core.
    //
    // The service is the supervisor, so the face only says what the
    // supervisor reports: a channel that could not bind is `failed` on
    // this terminal and in the document every surface reads, rather than
    // a "live" line over a socket nothing holds.
    // The setting is the door and the flag is an override of it: a home
    // that has configured `channels.enabled` gets the service with no
    // flag, and a flag starts it on a home that has not. Off either way
    // means no socket binds.
    let channel_config = crate::channels::ServiceConfig::from_settings(
        &home,
        if options.channels { Some(true) } else { None },
    );
    let _channels = if channel_config.enabled {
        let config = channel_config;
        let policy = crate::channels::Policy::new(home.clone());
        let driver = Arc::new(crate::channels::Driver::new(Arc::clone(&bridge), policy));
        let service = Arc::new(crate::channels::ChannelService::new(
            Arc::clone(&bridge),
            driver,
            &config,
        ));
        println!("fleet gui — the channel service is starting");
        let reports = service.start_all().await;
        let failed: Vec<&str> = reports
            .iter()
            .filter(|report| report.phase == crate::channels::Phase::Failed)
            .map(|report| report.name.as_str())
            .collect();
        if !failed.is_empty() {
            println!(
                "  {} channel{} did not start; `fleet channels` says why",
                failed.len(),
                if failed.len() == 1 { "" } else { "s" }
            );
        }
        service.spawn_watcher();
        service.spawn_heartbeat();
        // The control plane's start and stop reach the process that owns
        // the ports: the service is attached to the bridge the server
        // already answers from.
        bridge.attach_channels(Arc::clone(&service));
        Some(service)
    } else {
        None
    };

    // The relay: Fleet dials out so a phone on cellular can reach the
    // remote face with nothing installed on the network. The relay is a
    // transport, not a second implementation: the frames it forwards are
    // answered by the remote listener's own router.
    //
    // The shutdown sender is declared out here, not inside the block: a
    // binding scoped to the block drops the moment the block ends, and
    // the loop's `changed()` then answers at once — a poll task leaked
    // per iteration, the blocking pool saturated at 512 threads.
    let mut _relay_shutdown: Option<tokio::sync::watch::Sender<bool>> = None;
    if let Some(relay_url) = &options.relay {
        let Some(listener) = remote.as_ref() else {
            return Err(
                "fleet gui --relay needs --remote: a relay with no remote face has nothing to forward"
                    .to_string(),
            );
        };
        // The relay session is this launch's own: a session id and a
        // secret minted at bind, joined at the relay, and handed to the
        // phone through URLs whose secret part is a fragment browsers
        // never send. The frame keys come from the X25519 exchange the
        // handshake runs through the relay — the keys live here, shared
        // between the serve loop that seals and the handler that
        // answers the handshake.
        let sid = listener.device_session();
        let hash = listener.device_secret();
        let keys = Arc::new(std::sync::Mutex::new(
            crate::remote::relay::RelayKeys::mint(&sid),
        ));
        let client = Arc::new(crate::remote::relay::Client::new(relay_url, sid, hash));
        let handler = Arc::new(listener::RelayHandler::new(
            Arc::clone(&bridge),
            "a relayed phone".to_string(),
            listener::RemoteFace::Console,
            bridge.home_path(),
            Arc::clone(&keys),
            listener.device_secret(),
        ));
        // The pairing card folds the same secret into its QR's URL as a
        // fragment, so a phone that pairs through the relay origin can
        // name the relay and seal its frames.
        bridge.attach_relay_secret(listener.device_secret());
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        _relay_shutdown = Some(shutdown);
        let dialing = Arc::clone(&client);
        tokio::spawn(async move {
            crate::remote::relay::serve(dialing, handler, shutdown_rx, keys).await
        });
        println!("fleet gui — dialing the relay at {relay_url}");
        // The entry point as text as well as a QR: a phone that cannot
        // scan (a screenshot, a copy-paste) still has a way in. The
        // secret rides the fragment — browsers never send it — so the
        // edge and the relay log a URL that names nothing.
        println!(
            "  phone: {}/#k={}",
            relay_url.trim_end_matches('/'),
            listener.device_secret()
        );
        // The sender now lives in `_relay_shutdown`, held for the
        // process's life: a relay that stops when the face stops is the
        // honest shape.
    }

    // The bot's own relay: a separate service, its own origin, the same
    // seal. The owner's 2026-10-29 ruling — both faces reach the phone —
    // with the shape that adds no mechanism: the relay stays a
    // single-device switchboard, so a second deployment of the same two
    // files carries the bot face and nothing else.
    let mut _bot_relay_shutdown: Option<tokio::sync::watch::Sender<bool>> = None;
    if let Some(bot_relay_url) = &options.remote.bot_relay {
        // The credentials were captured at bind, before the bot
        // listener's serve task moved it: one session, one secret, minted
        // for the bot face alone.
        let Some((bot_sid, bot_hash)) = &bot_relay_credentials else {
            return Err(
                "fleet gui --bot-relay needs --remote: a relay with no bot face has nothing to forward"
                    .to_string(),
            );
        };
        let keys = Arc::new(std::sync::Mutex::new(
            crate::remote::relay::RelayKeys::mint(bot_sid),
        ));
        let client = Arc::new(crate::remote::relay::Client::new(
            bot_relay_url,
            bot_sid.clone(),
            bot_hash.clone(),
        ));
        let handler = Arc::new(listener::RelayHandler::new(
            Arc::clone(&bridge),
            "a relayed phone".to_string(),
            listener::RemoteFace::Bot,
            bridge.home_path(),
            Arc::clone(&keys),
            bot_hash.clone(),
        ));
        bridge.attach_bot_relay(bot_relay_url.clone(), bot_hash.clone());
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        _bot_relay_shutdown = Some(shutdown);
        let dialing = Arc::clone(&client);
        tokio::spawn(async move {
            crate::remote::relay::serve(dialing, handler, shutdown_rx, keys).await
        });
        println!("fleet gui — dialing the bot relay at {bot_relay_url}");
        println!(
            "  bot:    {}/#k={}  (the persona experience; pair it at its own origin)",
            bot_relay_url.trim_end_matches('/'),
            bot_hash
        );
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
