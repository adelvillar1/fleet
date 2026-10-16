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
//! - **One token per launch**, minted from the operating system's
//!   entropy and printed with the URL. Every route requires it — the
//!   header, the query, or the cookie — so there is no anonymous page,
//!   no anonymous asset, and no anonymous stream. A port scan sees
//!   401s.
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
pub mod server;

use std::process::Command;
use std::sync::Arc;

/// What `fleet gui` is told to do.
pub struct GuiOptions {
    /// The address to bind. Validated loopback before a socket exists.
    pub host: String,
    /// The port to bind; zero asks the operating system for a free one.
    pub port: u16,
    /// Open the browser anyway? `true` means do not: the URL and token
    /// are printed, and the owner opens the page themselves.
    pub no_open: bool,
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
/// the process. The URL carries the token because the browser needs it
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
    let bridge = bridge::Bridge::spawn(
        home,
        crate::api::DEFAULT_MODEL.to_string(),
        session.id().to_string(),
        base_url,
    );
    let (server, _port) =
        server::Server::bind(&options.host, options.port, Arc::new(bridge)).await?;
    let url = format!("{}?token={}", server.base_url(), server.token());
    println!("fleet gui — the browser face is live");
    println!("  {url}");
    println!("  Ctrl-C stops it.");
    if options.no_open {
        println!("  --no-open: the browser was left alone; open the URL above.");
    } else if let Err(e) = open_browser(&url) {
        // A browser that will not open is a convenience lost, not a
        // failure: the URL is on the screen and the server is live.
        println!("  could not open a browser ({e}); open the URL above.");
    }
    server.serve().await?;
    println!("fleet gui — stopped.");
    Ok(())
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
