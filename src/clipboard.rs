//! The clipboard, read on the owner's gesture.
//!
//! Two doors onto one pipeline, and their scopes differ. **⌘V** is the
//! primary: the page's own `paste` handler reads `clipboardData.items` and
//! attaches every image the event carries, with no server round trip. This
//! module is the SECOND door — the paste button — for a shell whose webview
//! refuses to hand an image to the page, which was the reason it was built.
//! The page's handler is the measurement that decides which is load-bearing
//! (the 2026-11-18 plan's C8): the handler demonstrably receives image items,
//! so a Chromium-backed webview does hand them over.
//!
//! Either way the reply shape is one list, because "paste one or more images"
//! is the requirement: a single image and a list of them feed the identical
//! chip, and the surface never special-cases a count.
//!
//! This module is that read and nothing else. It deliberately does not
//! decide what an image is *for*: the caller attaches it to a turn, and
//! the route's own bounds (validated in `gui::server`) decide whether it
//! fits. Keeping the read separate is what lets the tests drive it with
//! fixtures instead of reaching into the owner's pasteboard.
//!
//! **The failure speaks.** A clipboard that cannot be read is named as
//! such — a locked session, a headless process with no window — rather
//! than returned as an empty string that reads like "you copied
//! nothing".

use std::path::PathBuf;

/// What the clipboard holds, as this surface can carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    /// The clip holds text, which inserts into the composer.
    Text(String),
    /// The clip holds an image, which attaches to the turn.
    Image {
        /// A file name the attachment can carry, built from the type.
        name: String,
        /// The media type the pasteboard declared (or guessed from the
        /// bytes when it declared none).
        mime: String,
        /// The bytes, exactly as the pasteboard held them.
        bytes: Vec<u8>,
    },
    /// The clip holds SEVERAL images (the owner's 2026-11-18 request: "paste
    /// one or more images"). Each entry is the same shape one image uses, so
    /// the surface loops one pipeline rather than special-casing a list.
    Images(Vec<Image>),
    /// The clipboard holds neither, and saying so is the whole reply.
    Empty,
}

/// One image off the pasteboard: the same four fields whichever route found
/// it, so a list and a single image feed the identical attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// A file name the attachment can carry, built from the type or the
    /// source file's own name.
    pub name: String,
    /// The media type the pasteboard declared (or sniffed from the bytes).
    pub mime: String,
    /// The bytes, exactly as the pasteboard or the file held them.
    pub bytes: Vec<u8>,
}

/// What the pasteboard handed back for an image: the raw bytes it holds
/// and the media type it declared, when it declared one. The type is
/// `None` on a pasteboard that publishes pixels without naming them, and
/// a `None` means the caller sniffs rather than trusts.
pub type RawImage = (Vec<u8>, Option<String>);

/// The clipboard's two operations, behind a seam so a test can supply a
/// world instead of the owner's pasteboard.
pub trait Backend {
    /// The clip holds text.
    fn text(&mut self) -> Result<Option<String>, String>;
    /// The clip holds an image: its bytes and the type the pasteboard
    /// declared, when it declared one.
    fn image(&mut self) -> Result<Option<RawImage>, String>;

    /// The files the pasteboard holds, when it holds a LIST of them.
    ///
    /// A Finder copy of several images is a URL list, not an aggregate
    /// picture — `arboard`'s `image()` reads the pasteboard's ONE TIFF item,
    /// which is why a multi-file copy could not be read as images before.
    /// The default is empty so a backend that cannot answer a list (and every
    /// fixture written before this) keeps working unchanged.
    fn files(&mut self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// The system clipboard, through `arboard`.
///
/// Held as one value per read so the pasteboard is opened once, not
/// once per question: on macOS each open is a window-server round trip
/// and the second one can see a different clip than the first if the
/// owner copied again in between.
pub struct System {
    inner: arboard::Clipboard,
}

impl System {
    /// Opens the system clipboard, naming the failure rather than
    /// returning a value that looks like an empty clip.
    pub fn open() -> Result<Self, String> {
        arboard::Clipboard::new()
            .map(|inner| Self { inner })
            .map_err(|e| format!("the clipboard could not be read: {e}"))
    }
}

impl Backend for System {
    fn text(&mut self) -> Result<Option<String>, String> {
        match self.inner.get_text() {
            Ok(text) if !text.trim().is_empty() => Ok(Some(text)),
            Ok(_) => Ok(None),
            // "no text" is an answer, not a failure; the clipboard is
            // allowed to hold only an image.
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(format!("the clipboard could not be read: {e}")),
        }
    }

    fn image(&mut self) -> Result<Option<(Vec<u8>, Option<String>)>, String> {
        match self.inner.get_image() {
            Ok(data) => {
                let encoded = encode_png(&data)?;
                Ok(Some((encoded, Some("image/png".to_string()))))
            }
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(format!("the clipboard could not be read: {e}")),
        }
    }

    /// The files the pasteboard lists — the multi-image route.
    ///
    /// A Finder copy of several images is a URL list on the pasteboard, NOT
    /// an aggregate picture: `arboard`'s `image()` reads the pasteboard's one
    /// TIFF item, which is exactly why one image was all this could ever
    /// carry. `file_list()` reads `NSPasteboardURLReadingFileURLsOnlyKey` on
    /// macOS and the equivalent elsewhere, so a copy of six PNGs arrives as
    /// six paths.
    ///
    /// A pasteboard that holds no file list is `ContentNotAvailable`, which
    /// is an answer ("this clip is not a file copy"), not a failure.
    fn files(&mut self) -> Result<Vec<PathBuf>, String> {
        match self.inner.get().file_list() {
            Ok(paths) => Ok(paths),
            Err(arboard::Error::ContentNotAvailable) => Ok(Vec::new()),
            // The platform may refuse the file-list question outright (a
            // build without the feature, a surface that will not answer).
            // That is "no list", not a broken clipboard: the single-image
            // and text arms still run, so the paste degrades rather than
            // failing.
            Err(arboard::Error::ConversionFailure) => Ok(Vec::new()),
            Err(e) => Err(format!("the clipboard could not be read: {e}")),
        }
    }
}

/// Re-encodes the pasteboard's raw RGBA as PNG.
///
/// `arboard` hands back pixels, not a file: width, height and one byte
/// per channel. A model wants a media type it recognizes, and PNG is the
/// one every multimodal provider accepts, so the pixels are encoded here
/// rather than shipped as a raw buffer the other end cannot name.
fn encode_png(data: &arboard::ImageData) -> Result<Vec<u8>, String> {
    let width = u32::try_from(data.width).map_err(|_| "the clipboard image is too wide")?;
    let height = u32::try_from(data.height).map_err(|_| "the clipboard image is too tall")?;
    let expected = data.width * data.height * 4;
    if data.bytes.len() != expected {
        return Err(format!(
            "the clipboard image is {} bytes but {}x{} needs {expected}",
            data.bytes.len(),
            data.width,
            data.height
        ));
    }
    let mut png: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("the clipboard image could not be encoded: {e}"))?;
        writer
            .write_image_data(&data.bytes)
            .map_err(|e| format!("the clipboard image could not be encoded: {e}"))?;
    }
    Ok(png)
}

/// Reads the clipboard and says which kind of clip it found.
///
/// Text is asked first: a screenshot tool that also copies its own
/// caption, or a copied table, should insert as text where it can,
/// because text is what the composer can edit. An image is the answer
/// only when there is no text to carry.
pub fn read(backend: &mut dyn Backend) -> Result<Clip, String> {
    // IMAGES FIRST, and this order is the ruling rather than a preference
    // (the owner's 2026-11-18 request). A Finder copy hands the pasteboard
    // BOTH a file list and the names as text, so asking for text first pasted
    // `screenshot.png` instead of the picture. A person who copies an image
    // wants the image; the name is a representation, not the thing.
    let listed = read_files(backend)?;
    if !listed.is_empty() {
        if listed.len() == 1 {
            return Ok(Clip::Image {
                name: listed[0].name.clone(),
                mime: listed[0].mime.clone(),
                bytes: listed[0].bytes.clone(),
            });
        }
        return Ok(Clip::Images(listed));
    }
    if let Some((bytes, declared)) = backend.image()? {
        let mime = declared.unwrap_or_else(|| "image/png".to_string());
        return Ok(Clip::Image {
            name: format!("clipboard.{}", extension_for(&mime)),
            mime,
            bytes,
        });
    }
    // Text last: with no image anywhere, the ordinary office paste.
    if let Some(text) = backend.text()? {
        return Ok(Clip::Text(text));
    }
    Ok(Clip::Empty)
}

/// The images a file list names, each read and typed by its bytes.
///
/// A copied PDF beside two PNGs attaches the two and skips the third: the
/// file list is what the OS offers, not what the composer can carry, so a
/// non-image is passed over here rather than surfacing as a broken chip. The
/// bytes decide the type — an extension is a name, and the server re-checks
/// the declared type against the bytes anyway.
fn read_files(backend: &mut dyn Backend) -> Result<Vec<Image>, String> {
    let mut out: Vec<Image> = Vec::new();
    for path in backend.files()? {
        if path.is_dir() {
            continue;
        }
        let Some(mime) = sniff_type(&path) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("clipboard.{}", extension_for(mime)));
        out.push(Image {
            name,
            mime: mime.to_string(),
            bytes,
        });
    }
    Ok(out)
}

/// One path's media type by its FIRST BYTES, not its extension.
fn sniff_type(path: &std::path::Path) -> Option<&'static str> {
    use std::io::Read;
    let mut head = [0u8; 12];
    let mut file = std::fs::File::open(path).ok()?;
    let read = file.read(&mut head).ok()?;
    let head = &head[..read];
    if head.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if head.len() >= 12 && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

/// One image as the page's attachment shape: its name, type, base64 bytes and
/// size. One function, so a single image and a list entry cannot drift.
pub fn encoded(name: &str, mime: &str, bytes: &[u8]) -> serde_json::Value {
    use base64::Engine;
    serde_json::json!({
        "name": name,
        "mime": mime,
        "bytes": base64::engine::general_purpose::STANDARD.encode(bytes),
        "size": bytes.len(),
    })
}

/// The file extension a media type maps to.
fn extension_for(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clipboard that holds whatever a test puts in it.
    struct Fixture {
        text: Option<String>,
        image: Option<(Vec<u8>, Option<String>)>,
        /// The paths a file-list copy names. Empty is "not a file copy".
        files: Vec<std::path::PathBuf>,
    }

    impl Backend for Fixture {
        fn text(&mut self) -> Result<Option<String>, String> {
            Ok(self.text.clone())
        }
        fn image(&mut self) -> Result<Option<(Vec<u8>, Option<String>)>, String> {
            Ok(self.image.clone())
        }
        fn files(&mut self) -> Result<Vec<std::path::PathBuf>, String> {
            Ok(self.files.clone())
        }
    }

    fn holding(text: Option<&str>, image: Option<(Vec<u8>, Option<&str>)>) -> Fixture {
        Fixture {
            text: text.map(str::to_string),
            image: image.map(|(bytes, mime)| (bytes, mime.map(str::to_string))),
            files: Vec::new(),
        }
    }

    #[test]
    fn a_text_clip_is_the_text() {
        let mut clip = holding(Some("a pasted paragraph"), None);
        assert_eq!(
            read(&mut clip).expect("a readable clip"),
            Clip::Text("a pasted paragraph".to_string())
        );
    }

    #[test]
    fn an_image_clip_carries_its_type_and_a_name_the_attachment_can_hold() {
        let mut clip = holding(
            None,
            Some((vec![0x89, b'P', b'N', b'G'], Some("image/png"))),
        );
        let read_back = read(&mut clip).expect("a readable clip");
        assert_eq!(
            read_back,
            Clip::Image {
                name: "clipboard.png".to_string(),
                mime: "image/png".to_string(),
                bytes: vec![0x89, b'P', b'N', b'G'],
            }
        );
    }

    #[test]
    fn an_undeclared_image_type_is_read_as_png() {
        // A pasteboard that hands pixels without naming a type is the
        // macOS screenshot case; the name has to be usable, not empty.
        let mut clip = holding(None, Some((vec![1, 2, 3], None)));
        let read_back = read(&mut clip).expect("a readable clip");
        assert_eq!(
            read_back,
            Clip::Image {
                name: "clipboard.png".to_string(),
                mime: "image/png".to_string(),
                bytes: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn a_jpeg_names_itself_jpg() {
        let mut clip = holding(None, Some((vec![0xff, 0xd8], Some("image/jpeg"))));
        let read_back = read(&mut clip).expect("a readable clip");
        match read_back {
            Clip::Image { name, mime, .. } => {
                assert_eq!(name, "clipboard.jpg");
                assert_eq!(mime, "image/jpeg");
            }
            other => panic!("a jpeg is an image, got {other:?}"),
        }
    }

    #[test]
    fn an_image_wins_over_text_because_text_is_the_image_s_name() {
        // The precedence FLIPPED on the owner's 2026-11-18 ruling. A Finder
        // copy puts both the file list and the names on the pasteboard, so
        // asking for text first pasted `screenshot.png` instead of the
        // picture. A person who copies an image wants the image; the name is
        // a representation, not the thing.
        let mut clip = holding(Some("screenshot.png"), Some((vec![9], Some("image/png"))));
        assert_eq!(
            read(&mut clip).expect("a readable clip"),
            Clip::Image {
                name: "clipboard.png".to_string(),
                mime: "image/png".to_string(),
                bytes: vec![9],
            }
        );
        // With no image anywhere, the ordinary office paste still wins.
        let mut text_only = holding(Some("a pasted paragraph"), None);
        assert_eq!(
            read(&mut text_only).expect("a readable clip"),
            Clip::Text("a pasted paragraph".to_string())
        );
    }

    #[test]
    fn a_file_list_becomes_every_image_it_names() {
        // The multi-image route: a Finder copy of several PNGs is a URL list,
        // not an aggregate picture (which is why one image was all this could
        // ever carry). Real files on disk, because the reader sniffs their
        // bytes rather than trusting an extension.
        let dir = std::env::temp_dir().join(format!("fleet-clip-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let png = |name: &str, tail: u8| {
            let path = dir.join(name);
            let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            bytes.extend_from_slice(&[tail; 8]);
            std::fs::write(&path, &bytes).unwrap();
            path
        };
        let a = png("a.png", 1);
        let b = png("b.png", 2);
        // A non-image in the same copy: named by nobody, attached by nobody.
        let note = dir.join("notes.txt");
        std::fs::write(&note, b"just text").unwrap();

        let mut clip = holding(None, None);
        clip.files = vec![a, b, note];
        let read_back = read(&mut clip).expect("a readable clip");
        match read_back {
            Clip::Images(images) => {
                assert_eq!(images.len(), 2, "the two PNGs, not the txt: {images:?}");
                assert_eq!(images[0].name, "a.png");
                assert_eq!(images[0].mime, "image/png");
                assert_eq!(images[1].name, "b.png");
            }
            other => panic!("expected a list, got {other:?}"),
        }
        // One file is the SINGLE image shape, so the surface keeps one path
        // for the common case and the list is the many case.
        let mut one = holding(None, None);
        one.files = vec![png("only.png", 3)];
        assert!(matches!(
            read(&mut one).expect("a readable clip"),
            Clip::Image { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_clipboard_says_so_rather_than_inventing_a_clip() {
        let mut clip = holding(None, None);
        assert_eq!(read(&mut clip).expect("a readable clip"), Clip::Empty);
    }

    #[test]
    fn the_pasteboards_pixels_are_encoded_as_a_real_png() {
        // A 2x2 red square, the smallest thing that is unmistakably an
        // image: arboard hands RGBA, and the other end must receive a
        // file it can name as image/png.
        let data = arboard::ImageData {
            width: 2,
            height: 2,
            bytes: std::borrow::Cow::Owned([255u8, 0, 0, 255].repeat(4)),
        };
        let png = encode_png(&data).expect("the pixels encode");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "the PNG signature");
        // The IHDR carries the dimensions, big-endian, at byte 16.
        assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 2]);
    }

    #[test]
    fn a_mismatched_pixel_buffer_is_named_rather_than_encoded() {
        let data = arboard::ImageData {
            width: 4,
            height: 4,
            bytes: std::borrow::Cow::Owned(vec![0; 12]),
        };
        let why = encode_png(&data).expect_err("a short buffer is refused");
        assert!(why.contains("12 bytes"), "{why}");
        assert!(why.contains("4x4"), "{why}");
    }
}
