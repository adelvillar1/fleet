//! The pane's web tab: fetch a page, extract what is readable, render it.
//!
//! The pane browses the web the way it browses files — server-side, with
//! the result rendered in this app's own typography rather than dropped
//! into a frame. That choice is not laziness; it is the only one that
//! works and the only one that is safe:
//!
//! **An iframe does not work.** Nearly every site worth reading sends
//! `X-Frame-Options: DENY` or a `frame-ancestors` CSP directive, so the
//! pane would show a blank rectangle for the pages the owner actually
//! wants. Fetching server-side and rendering the content sidesteps the
//! header entirely.
//!
//! **A server-side fetch is an SSRF vector**, so the guard is the point
//! of this module rather than a footnote to it. A browser that can name
//! any URL is a browser that can name `http://127.0.0.1:8901` (the
//! embedding model), `http://127.0.0.1:8420` (the blend1 lane), and
//! `http://169.254.169.254` (the cloud metadata service, which hands out
//! credentials). Every hop — the first and every redirect — is checked
//! against the scheme and the addresses the host resolves to, and a
//! private or loopback address is refused by name.
//!
//! What renders is text: the title, the headings, the paragraphs, the
//! lists, the links, the code. No script from the page runs, no external
//! resource loads, and nothing the page says about itself is trusted.

use std::io::Read;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

/// How long one fetch may take. A page that has not answered in ten
/// seconds is a page the owner did not want badly enough to wait for.
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// How many bytes of one page are read. The extractor only wants the
/// head of the document anyway — the readable content is nearly always
/// in the first hundred kilobytes — and an unbounded read is a memory
/// amplification the pane does not need.
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// How many redirects are followed. Each hop is re-validated, so the
/// number is a bound on the work rather than on the trust.
const MAX_REDIRECTS: usize = 5;

/// The user agent this fetch presents. Naming itself is the polite thing
/// and it is also what keeps a site from serving the pane a
/// bot-detection page.
const USER_AGENT: &str = concat!("fleet/", env!("CARGO_PKG_VERSION"), " (loopback reader)");

/// Why a fetch was refused. Every variant names what was wrong rather
/// than returning an empty page that would read as "this site is blank".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The URL is not one this module will fetch.
    Scheme(String),
    /// The host does not resolve.
    Resolve(String),
    /// The host resolves to an address this module will not reach.
    Private(String),
    /// The transport failed.
    Transport(String),
    /// The page is larger than the read carries.
    TooLarge,
    /// Too many redirects.
    Redirects,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Scheme(url) => {
                write!(f, "`{url}` is not an http or https address")
            }
            FetchError::Resolve(host) => write!(f, "`{host}` does not resolve"),
            FetchError::Private(host) => write!(
                f,
                "`{host}` resolves to a private or loopback address, which this pane will not reach"
            ),
            FetchError::Transport(reason) => write!(f, "the fetch failed: {reason}"),
            FetchError::TooLarge => write!(
                f,
                "the page is larger than the {} bytes this pane reads",
                MAX_BODY_BYTES
            ),
            FetchError::Redirects => {
                write!(f, "the page redirected more than {MAX_REDIRECTS} times")
            }
        }
    }
}

/// Whether one address is one this pane will reach. Loopback, private,
/// link-local, unspecified, and the cloud metadata service are all out —
/// the first four are this machine and its neighbours, and the last hands
/// out credentials to anything that asks.
fn is_private(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets() == [169, 254, 169, 254]
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                // The v4-mapped range is where a literal IPv4 address
                // arrives inside a v6 record, so it is checked as v4.
                || v6
                    .to_ipv4_mapped()
                    .is_some_and(|v4| is_private(IpAddr::V4(v4)))
                // Unique-local (fc00::/7) and link-local (fe80::/10), the
                // v6 equivalents of private and link-local.
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Validate one URL and resolve its host, refusing anything this pane
/// will not reach. The address check is on the *resolved* addresses, not
/// the name, so a host that points at `127.0.0.1` is refused the same as
/// the literal.
fn guard(url: &str) -> Result<(String, String), FetchError> {
    let trimmed = url.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err(FetchError::Scheme(trimmed.to_string()));
    };
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::Scheme(trimmed.to_string()));
    }
    // The authority is everything up to the first slash, question mark or
    // hash; the host is that minus any userinfo and any port.
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();
    let authority = authority
        .rsplit('@')
        .next()
        .unwrap_or(&authority)
        .to_string();
    // An IPv6 literal is bracketed and its colons are not a port, so it
    // is split on the bracket rather than the colon.
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest).to_string()
    } else {
        authority
            .rsplit_once(':')
            .map(|(host, _)| host.to_string())
            .unwrap_or(authority)
    };
    if host.is_empty() {
        return Err(FetchError::Scheme(trimmed.to_string()));
    }
    // Resolve. A host that does not resolve is refused rather than
    // fetched, because the fetch would fail anyway and the refusal says
    // which of the two happened.
    let resolved: Vec<IpAddr> = match (host.as_str(), 0u16).to_socket_addrs() {
        Ok(addresses) => addresses.map(|address| address.ip()).collect(),
        Err(_) => return Err(FetchError::Resolve(host)),
    };
    if resolved.is_empty() {
        return Err(FetchError::Resolve(host));
    }
    if resolved.iter().any(|address| is_private(*address)) {
        return Err(FetchError::Private(host));
    }
    Ok((trimmed.to_string(), host))
}

/// One page, fetched.
#[derive(Debug, Clone)]
pub struct Page {
    /// The address the page was finally read from, after redirects —
    /// what the pane's address bar shows once the read lands.
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl Readable {
    /// The page as plain text, block by block, for a caller that shows
    /// prose rather than rendering it.
    ///
    /// The shape is markdown-ish on purpose: a heading keeps its level as
    /// `#`s, an item keeps its `-`, code keeps its fence. A model reading
    /// this gets the page's structure without a second renderer, and the
    /// text stays quotable — which is what a citation needs.
    pub fn plain_text(&self) -> String {
        let mut text = String::new();
        for block in &self.blocks {
            match block {
                Block::Heading {
                    level,
                    text: heading,
                } => {
                    text.push_str(&"#".repeat((*level).min(6) as usize));
                    text.push(' ');
                    text.push_str(heading);
                    text.push_str("\n\n");
                }
                Block::Paragraph(text_) => {
                    text.push_str(text_);
                    text.push_str("\n\n");
                }
                Block::Item(item) => {
                    text.push_str("- ");
                    text.push_str(item);
                    text.push('\n');
                }
                Block::Code(code) => {
                    text.push_str("```\n");
                    text.push_str(code);
                    text.push_str("\n```\n\n");
                }
                Block::Quote(quote) => {
                    text.push_str("> ");
                    text.push_str(quote);
                    text.push_str("\n\n");
                }
            }
        }
        text.trim_end().to_string()
    }
}

/// Fetch one page, following and re-validating every redirect.
/// The verdict a browser-in-the-pane needs before it renders an
/// address live: will the site allow being framed? The site's own
/// headers answer — `x-frame-options` and CSP `frame-ancestors` — and
/// the content type must be a document at all. The body is never read:
/// the headers are the whole answer.
pub struct FrameVerdict {
    /// The address after redirects — what the pane's address bar shows.
    pub url: String,
    pub framable: bool,
    /// Why not, in the site's own words, when `framable` is false.
    pub reason: String,
    pub content_type: String,
}

pub fn framable(url: &str) -> Result<FrameVerdict, FetchError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(FETCH_TIMEOUT)
        .redirects(0)
        .user_agent(USER_AGENT)
        .build();
    let mut current = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        let (checked, _host) = guard(&current)?;
        let response = agent
            .get(&checked)
            .call()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        let status = response.status();
        if (300..400).contains(&status) {
            let Some(location) = response.header("location") else {
                return Err(FetchError::Transport(format!(
                    "a {status} with no location to follow"
                )));
            };
            current = resolve_redirect(&current, location);
            continue;
        }
        let content_type = response
            .header("content-type")
            .unwrap_or_default()
            .to_string();
        let xfo = response
            .header("x-frame-options")
            .unwrap_or_default()
            .to_uppercase();
        let csp = response
            .header("content-security-policy")
            .unwrap_or_default()
            .to_lowercase();
        // A frame renders documents: HTML, or a server that names no
        // type (the lenient read — the frame itself will show what the
        // site serves). Anything download-shaped stays behind the
        // reader door.
        let is_document = content_type.is_empty() || content_type.starts_with("text/html");
        let denied_by_xfo = xfo.contains("DENY") || xfo.contains("SAMEORIGIN");
        let denied_by_csp = csp.split(';').any(|directive| {
            let directive = directive.trim();
            directive.starts_with("frame-ancestors")
                && !directive
                    .split_whitespace()
                    .any(|token| token == "*" || token == "http:" || token == "https:")
        });
        let framable = is_document && !denied_by_xfo && !denied_by_csp;
        let reason = if !is_document {
            format!("the address is a {content_type}, not a document to render")
        } else if denied_by_xfo {
            format!("the site's own x-frame-options says {xfo}")
        } else if denied_by_csp {
            "the site's content-security-policy forbids being framed".to_string()
        } else {
            String::new()
        };
        // The response drops here unread: the headers were the answer.
        return Ok(FrameVerdict {
            url: current,
            framable,
            reason,
            content_type,
        });
    }
    Err(FetchError::Transport("too many redirects".to_string()))
}

pub fn fetch(url: &str) -> Result<Page, FetchError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(FETCH_TIMEOUT)
        .redirects(0)
        .user_agent(USER_AGENT)
        .build();
    let mut current = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        let (checked, _host) = guard(&current)?;
        let response = agent
            .get(&checked)
            .call()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        let status = response.status();
        if (300..400).contains(&status) {
            let Some(location) = response.header("location") else {
                return Err(FetchError::Transport(format!(
                    "a {status} with no location to follow"
                )));
            };
            current = resolve_redirect(&current, location);
            continue;
        }
        let content_type = response
            .header("content-type")
            .unwrap_or_default()
            .to_string();
        let mut body = String::new();
        response
            .into_reader()
            .take(MAX_BODY_BYTES as u64 + 1)
            .read_to_string(&mut body)
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        if body.len() > MAX_BODY_BYTES {
            return Err(FetchError::TooLarge);
        }
        return Ok(Page {
            url: current,
            status,
            content_type,
            body,
        });
    }
    Err(FetchError::Redirects)
}

/// One redirect's target, made absolute against the page that sent it.
fn resolve_redirect(from: &str, location: &str) -> String {
    let location = location.trim();
    if location.contains("://") {
        return location.to_string();
    }
    let Some((scheme, rest)) = from.split_once("://") else {
        return location.to_string();
    };
    let origin_end = rest.find('/').unwrap_or(rest.len());
    let origin = format!("{scheme}://{}", &rest[..origin_end]);
    if let Some(path) = location.strip_prefix('/') {
        return format!("{origin}/{path}");
    }
    // A relative reference resolves against the current directory.
    let base = from
        .rsplit_once('/')
        .map(|(head, _)| head)
        .unwrap_or(&origin);
    format!("{base}/{location}")
}

// --- the readable extraction ------------------------------------------------

/// One block of readable content. The set is deliberately small — the
/// renderer in the pane draws exactly these — because a block the pane
/// cannot draw is a block the extractor should not have emitted.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    Item(String),
    Code(String),
    /// A quote.
    Quote(String),
}

/// One page, reduced to what is readable: the title, the prose, and the
/// links it carries.
///
/// A link's label stays in its sentence rather than being torn out of it —
/// a reader mode that leaves "the first paragraph, with an in it" is worse
/// than one that keeps the prose whole and lists the links beside it.
#[derive(Debug, Clone)]
pub struct Readable {
    pub title: String,
    pub blocks: Vec<Block>,
    /// `(label, href)` in the order the page carries them.
    pub links: Vec<(String, String)>,
}

/// The elements whose text is navigation, chrome, or advertising rather
/// than the page's content. Dropped before anything else is considered,
/// because a "readable" extraction that carries the cookie banner is not
/// one.
const DROP_ELEMENTS: &[&str] = &[
    "script", "style", "noscript", "svg", "canvas", "iframe", "nav", "header", "footer", "aside",
    "form", "button", "input", "select", "textarea", "label",
];

/// The elements that are themselves a block of content.
const BLOCK_ELEMENTS: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "main",
    "li",
    "ul",
    "ol",
    "blockquote",
    "pre",
    "code",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "tr",
    "table",
    "br",
    "hr",
    "figcaption",
    "figure",
    "dl",
    "dt",
    "dd",
    "details",
    "summary",
];

/// Reduce one HTML document to its readable content.
///
/// This is not a full HTML parser — it is a tag-and-text scanner that
/// knows enough about the shape of a document to drop its chrome and keep
/// its prose. A page whose markup defeats it renders as its visible text
/// rather than failing, which is the honest degradation: something readable
/// is better than an error the owner cannot act on.
pub fn readable(html: &str) -> Readable {
    let mut title = String::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph = String::new();
    let mut code: Option<String> = None;
    let mut quote: Option<String> = None;
    let mut drop_depth: Vec<String> = Vec::new();
    // The open link: where in the paragraph its label starts, and where
    // it goes.
    let mut link: Option<(usize, String)> = None;
    let mut links: Vec<(String, String)> = Vec::new();
    let mut at = 0usize;
    let bytes = html.as_bytes();

    let flush_paragraph = |paragraph: &mut String, blocks: &mut Vec<Block>| {
        let text = collapse(paragraph);
        if !text.is_empty() {
            blocks.push(Block::Paragraph(text));
        }
        paragraph.clear();
    };

    while at < bytes.len() {
        if bytes[at] != b'<' {
            // A run of text up to the next tag.
            let end = html[at..]
                .find('<')
                .map(|offset| at + offset)
                .unwrap_or(bytes.len());
            let text = unescape(&html[at..end]);
            if drop_depth.is_empty() {
                if let Some(code) = code.as_mut() {
                    code.push_str(&text);
                } else if let Some(quote) = quote.as_mut() {
                    quote.push_str(&text);
                } else {
                    paragraph.push_str(&text);
                }
            }
            at = end;
            continue;
        }
        // A tag. A comment is skipped whole.
        if html[at..].starts_with("<!--") {
            at = html[at..]
                .find("-->")
                .map(|offset| at + offset + 3)
                .unwrap_or(bytes.len());
            continue;
        }
        let Some(end) = html[at..].find('>').map(|offset| at + offset) else {
            break;
        };
        let raw = &html[at + 1..end];
        at = end + 1;
        let closing = raw.starts_with('/');
        let inner = raw.trim_start_matches('/');
        let (name, attrs) = match inner.split_once(|c: char| c.is_whitespace()) {
            Some((name, attrs)) => (name.to_ascii_lowercase(), attrs.to_string()),
            None => (inner.to_ascii_lowercase(), String::new()),
        };
        let name = name.trim_end_matches('/').to_string();

        if closing {
            if name == "blockquote" {
                if let Some(text) = quote.take() {
                    let text = collapse(&text);
                    if !text.is_empty() {
                        blocks.push(Block::Quote(text));
                    }
                }
                continue;
            }
            if (name == "pre" || name == "code") && code.is_some() {
                if let Some(text) = code.take() {
                    if !text.trim().is_empty() {
                        blocks.push(Block::Code(text));
                    }
                }
                continue;
            }
            if let Some((start, href)) = link.take() {
                if name == "a" && paragraph.len() > start {
                    // The label is the span of the sentence the link was
                    // made of. It stays where it is — the prose reads
                    // whole — and the link is listed beside it.
                    let label = collapse(&paragraph[start..]);
                    if !label.is_empty() {
                        links.push((label, href));
                    }
                    continue;
                }
            }
            if let Some(position) = drop_depth.iter().rposition(|open| *open == name) {
                drop_depth.truncate(position);
                continue;
            }
            if name == "li" || name == "dt" || name == "dd" {
                flush_paragraph(&mut paragraph, &mut blocks);
                // The paragraph that just closed is the item's text.
                if let Some(Block::Paragraph(text)) = blocks.pop() {
                    blocks.push(Block::Item(text));
                }
                continue;
            }
            if BLOCK_ELEMENTS.contains(&name.as_str()) {
                flush_paragraph(&mut paragraph, &mut blocks);
            }
            continue;
        }

        // An opening tag.
        if DROP_ELEMENTS.contains(&name.as_str()) {
            drop_depth.push(name);
            continue;
        }
        if !drop_depth.is_empty() {
            continue;
        }
        match name.as_str() {
            "title" => {
                let close = "</title";
                if let Some(offset) = html[at..].find(close) {
                    title = unescape(&strip_tags(&html[at..at + offset]));
                    at += offset + close.len();
                    if let Some(gt) = html[at..].find('>') {
                        at += gt + 1;
                    }
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                flush_paragraph(&mut paragraph, &mut blocks);
                let level = name.as_bytes()[1] - b'0';
                let mut text = String::new();
                // Read to the matching close, so a heading with a nested
                // tag still comes out as one line.
                let close = format!("</{name}");
                if let Some(offset) = html[at..].find(&close) {
                    text = unescape(&strip_tags(&html[at..at + offset]));
                    at += offset + close.len();
                    if let Some(gt) = html[at..].find('>') {
                        at += gt + 1;
                    }
                }
                let text = collapse(&text);
                if !text.is_empty() {
                    blocks.push(Block::Heading { level, text });
                }
            }
            "pre" => {
                flush_paragraph(&mut paragraph, &mut blocks);
                code = Some(String::new());
            }
            "a" => {
                link = Some((
                    paragraph.len(),
                    attribute(&attrs, "href").unwrap_or_default(),
                ));
            }
            "blockquote" => {
                flush_paragraph(&mut paragraph, &mut blocks);
                quote = Some(String::new());
            }
            "br" | "hr" => flush_paragraph(&mut paragraph, &mut blocks),
            "li" => flush_paragraph(&mut paragraph, &mut blocks),
            "dt" | "dd" => flush_paragraph(&mut paragraph, &mut blocks),
            _ => {}
        }
    }
    if let Some(code) = code.take() {
        if !code.trim().is_empty() {
            blocks.push(Block::Code(code));
        }
    }
    if let Some(text) = quote.take() {
        let text = collapse(&text);
        if !text.is_empty() {
            blocks.push(Block::Quote(text));
        }
    }
    flush_paragraph(&mut paragraph, &mut blocks);

    Readable {
        title: collapse(&title),
        blocks,
        links,
    }
}

/// One attribute's value off a tag's attribute string.
fn attribute(attrs: &str, want: &str) -> Option<String> {
    for part in attrs.split_whitespace() {
        if let Some((name, value)) = part.split_once('=') {
            if name.eq_ignore_ascii_case(want) {
                return Some(value.trim_matches(['"', '\'']).to_string());
            }
        }
    }
    None
}

/// Strip every tag from a fragment, keeping its text.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut at = 0usize;
    while let Some(offset) = html[at..].find('<') {
        out.push_str(&html[at..at + offset]);
        let end = at + offset;
        match html[end..].find('>') {
            Some(close) => at = end + close + 1,
            None => break,
        }
    }
    out.push_str(&html[at..]);
    out
}

/// Unescape the five entities HTML requires plus the numeric forms,
/// which is what a page's prose actually carries.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(offset) = rest.find('&') {
        out.push_str(&rest[..offset]);
        let after = &rest[offset..];
        // An entity is short; anything past a dozen bytes is an ampersand
        // that happens to be in the prose.
        let Some(semicolon) = after.find(';').filter(|at| *at <= 12) else {
            out.push('&');
            rest = &rest[offset + 1..];
            continue;
        };
        let entity = &after[..semicolon];
        let replacement: Option<String> = match entity {
            "&amp" => Some("&".to_string()),
            "&lt" => Some("<".to_string()),
            "&gt" => Some(">".to_string()),
            "&quot" => Some("\"".to_string()),
            "&apos" => Some("'".to_string()),
            "&nbsp" => Some("\u{a0}".to_string()),
            other => other
                .strip_prefix("&#")
                .and_then(|digits| {
                    let code = match digits
                        .strip_prefix('x')
                        .or_else(|| digits.strip_prefix('X'))
                    {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => digits.parse().ok(),
                    };
                    code
                })
                .and_then(char::from_u32)
                .map(|character| character.to_string()),
        };
        match replacement {
            Some(text) => {
                out.push_str(&text);
                // Past the semicolon, not onto it.
                rest = &rest[offset + semicolon + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[offset + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Collapse whitespace, the way a reader expects prose to arrive.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A redirect is re-guarded, so the address a page sends the reader
    /// to is held to the same rule as the address that was asked for.
    ///
    /// `fetch` calls `guard` once per hop, on the URL it is about to
    /// read; this pins the composition the loop performs — a redirect
    /// target resolved against the page that sent it, then guarded — for
    /// the hop that matters most: a public page pointing at the metadata
    /// service, which is how an SSRF guard is usually walked around.
    #[test]
    fn a_redirect_to_a_private_address_fails_the_next_hop_s_guard() {
        for (from, location) in [
            (
                "https://example.com/report",
                "http://169.254.169.254/latest/meta-data/",
            ),
            (
                "https://example.com/report",
                "http://127.0.0.1:8901/v1/embeddings",
            ),
            ("https://example.com/report", "/../internal"),
            ("https://example.com/a/b", "http://10.0.0.7/"),
        ] {
            let next = resolve_redirect(from, location);
            match guard(&next) {
                Err(FetchError::Private(_)) => {}
                // `/../internal` stays on the same public origin: the
                // point of that case is that a relative hop is resolved,
                // not that it leaves the host.
                Ok(_) if location.starts_with('/') => {}
                other => panic!("{next} was not refused: {other:?}"),
            }
        }
    }

    /// The guard refuses everything that is not an http(s) address, and
    /// everything that resolves to this machine or its neighbours — the
    /// SSRF boundary, which is the reason this module exists.
    #[test]
    fn the_guard_refuses_private_and_loopback_addresses() {
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/",
            "gopher://example.com/",
            "http://127.0.0.1:8901/v1/embeddings",
            "http://localhost:3192/api/state",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://[::1]:8420/",
            "http://0.0.0.0/",
        ] {
            assert!(
                matches!(
                    guard(url),
                    Err(FetchError::Scheme(_)) | Err(FetchError::Private(_))
                ),
                "`{url}` was allowed"
            );
        }
        // A public address passes the guard (it is not fetched here).
        assert!(
            guard("https://example.com/").is_ok(),
            "a public address was refused"
        );
    }

    /// A relative redirect resolves against the page that sent it, and an
    /// absolute one is taken as it stands.
    #[test]
    fn redirects_resolve_against_the_page_that_sent_them() {
        assert_eq!(
            resolve_redirect("https://example.com/a/b", "/c"),
            "https://example.com/c"
        );
        assert_eq!(
            resolve_redirect("https://example.com/a/b", "c"),
            "https://example.com/a/c"
        );
        assert_eq!(
            resolve_redirect("https://example.com/a/b", "https://other.example/x"),
            "https://other.example/x"
        );
    }

    /// The extractor keeps the prose and drops the chrome.
    #[test]
    fn the_extractor_keeps_prose_and_drops_chrome() {
        let html = r#"<!doctype html>
<html><head><title>A Page About Reading</title>
<script>var tracking = true;</script>
<style>body { color: red }</style></head>
<body>
<nav><a href="/home">Home</a> <a href="/about">About</a></nav>
<header><h1>Site Banner</h1></header>
<main>
<h2>The first section</h2>
<p>The first paragraph, with an <a href="/link">inline link</a> in it.</p>
<p>A second paragraph &amp; an entity.</p>
<ul><li>One item</li><li>Two items</li></ul>
<pre><code>let x = 1;</code></pre>
<blockquote>A quoted line.</blockquote>
</main>
<footer>Copyright notices</footer>
<aside>Related reading</aside>
</body></html>"#;
        let readable = readable(html);
        assert_eq!(readable.title, "A Page About Reading");
        let kinds: Vec<&str> = readable
            .blocks
            .iter()
            .map(|block| match block {
                Block::Heading { .. } => "heading",
                Block::Paragraph(_) => "paragraph",
                Block::Item(_) => "item",
                Block::Code(_) => "code",
                Block::Quote(_) => "quote",
            })
            .collect();
        // The nav, the banner, the footer and the aside are gone; the
        // main's content is here in order.
        assert!(kinds.contains(&"heading"), "{readable:?}");
        assert!(kinds.contains(&"paragraph"), "{readable:?}");
        assert!(kinds.contains(&"item"), "{readable:?}");
        assert!(kinds.contains(&"code"), "{readable:?}");
        assert!(kinds.contains(&"quote"), "{readable:?}");
        let text = format!("{readable:?}");
        assert!(!text.contains("tracking"), "a script's text survived");
        assert!(!text.contains("color: red"), "a style's text survived");
        assert!(!text.contains("Site Banner"), "the header survived");
        assert!(!text.contains("Copyright notices"), "the footer survived");
        assert!(!text.contains("Related reading"), "the aside survived");
        assert!(!text.contains("Home"), "the nav survived");
        // The entity is decoded, and the sentence keeps the words the
        // link was made of rather than losing them.
        assert!(
            text.contains("A second paragraph & an entity"),
            "{readable:?}"
        );
        assert!(
            text.contains("The first paragraph, with an inline link in it."),
            "{readable:?}"
        );
        // The link itself is listed beside the prose, with its target.
        assert_eq!(
            readable.links,
            vec![("inline link".to_string(), "/link".to_string())],
            "{readable:?}"
        );
    }

    /// Entities decode, including the numeric forms.
    #[test]
    fn entities_decode() {
        assert_eq!(
            unescape("a &amp; b &lt;c&gt; &quot;d&quot;"),
            "a & b <c> \"d\""
        );
        assert_eq!(unescape("caf&#233;"), "caf\u{e9}");
        assert_eq!(unescape("caf&#xE9;"), "caf\u{e9}");
        assert_eq!(unescape("no entities here"), "no entities here");
    }

    /// A page with no markup at all still reads as its text.
    #[test]
    fn a_plain_text_page_reads_as_its_text() {
        let readable = readable("Just some prose, with no tags in it at all.");
        assert_eq!(
            readable.blocks,
            vec![Block::Paragraph(
                "Just some prose, with no tags in it at all.".to_string()
            )]
        );
    }

    /// Whitespace collapses, so a document indented for readability does
    /// not arrive as a wall of spaces.
    #[test]
    fn whitespace_collapses() {
        let readable = readable("<p>  one\n\n   two  </p>");
        assert_eq!(
            readable.blocks,
            vec![Block::Paragraph("one two".to_string())]
        );
    }
}
