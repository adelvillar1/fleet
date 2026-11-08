//! The clipboard, read on the owner's gesture.
//!
//! The composer's Cmd+V works for text and does nothing for an image.
//! That is not a bug in the handler: a webview does not hand an image to
//! the page at all, so the `paste` event either never fires or arrives
//! with an empty `clipboardData`. The only way to read what the owner
//! copied is to ask the system clipboard from the process that owns a
//! window — which is this one — and hand the result to the page over the
//! server the page already talks to.
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
    /// The clipboard holds neither, and saying so is the whole reply.
    Empty,
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
    if let Some(text) = backend.text()? {
        return Ok(Clip::Text(text));
    }
    if let Some((bytes, declared)) = backend.image()? {
        let mime = declared.unwrap_or_else(|| "image/png".to_string());
        let ext = match mime.as_str() {
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            _ => "png",
        };
        return Ok(Clip::Image {
            name: format!("clipboard.{ext}"),
            mime,
            bytes,
        });
    }
    Ok(Clip::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clipboard that holds whatever a test puts in it.
    struct Fixture {
        text: Option<String>,
        image: Option<(Vec<u8>, Option<String>)>,
    }

    impl Backend for Fixture {
        fn text(&mut self) -> Result<Option<String>, String> {
            Ok(self.text.clone())
        }
        fn image(&mut self) -> Result<Option<(Vec<u8>, Option<String>)>, String> {
            Ok(self.image.clone())
        }
    }

    fn holding(text: Option<&str>, image: Option<(Vec<u8>, Option<&str>)>) -> Fixture {
        Fixture {
            text: text.map(str::to_string),
            image: image.map(|(bytes, mime)| (bytes, mime.map(str::to_string))),
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
    fn text_wins_over_an_image_because_text_is_what_the_composer_edits() {
        let mut clip = holding(Some("words"), Some((vec![9], Some("image/png"))));
        assert_eq!(
            read(&mut clip).expect("a readable clip"),
            Clip::Text("words".to_string())
        );
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
