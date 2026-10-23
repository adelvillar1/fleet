//! The capability layer: the model names what it wants done, the
//! capability layer names the lane that does it.
//!
//! Three vendored lanes expose tools the model never sees — `sdm1`
//! measures a table, `gen1` renders an image, `browser` opens a page.
//! The capability tools are the model-facing surface of those: one
//! native declaration per capability, translated here into the lane
//! call the roster dispatches. A lane that cannot launch or names a
//! missing key is the lane's own failure riding the ordinary result
//! path — never a silent empty answer.

use serde_json::{json, Value};

/// The model-visible tool names.
pub const MEASURE_TOOL: &str = "measure";
pub const IMAGINE_TOOL: &str = "imagine";
pub const BROWSE_TOOL: &str = "browse";

/// The lane a capability dispatches to and the lane tool that serves
/// it: `(capability, lane, lane tool)`.
pub fn lane_for(capability: &str) -> Option<(&'static str, &'static str)> {
    match capability {
        MEASURE_TOOL => Some(("sdm1", "sdm1_classify")),
        IMAGINE_TOOL => Some(("gen1", "gen1_imagine")),
        BROWSE_TOOL => Some(("browser", "")),
        _ => None,
    }
}

/// The model-visible declaration for one capability tool.
pub fn spec(capability: &str) -> Option<crate::api::ToolSpec> {
    let (name, description, parameters) = match capability {
        MEASURE_TOOL => (
            MEASURE_TOOL,
            "Score a table with a tabular foundation model. Rows carrying a non-null `target` \
             are the labeled context; rows with `target` null are scored. Returns predictions \
             with confidence, the verdict, and the provider that answered. A measurement is a \
             computed number with an interval, never a guess.",
            json!({
                "type": "object",
                "properties": {
                    "table": {
                        "type": "array",
                        "items": { "type": "object" },
                        "description": "Rows as objects; a `target` column separates labeled context from query rows."
                    },
                    "target": { "type": "string", "description": "Name of the target column." },
                    "task_type": {
                        "type": "string",
                        "enum": ["classification", "regression", "forecast", "anomaly"],
                        "description": "Default classification. Forecast needs a time-ordered table; anomaly query rows carry the observed value in `<target>_actual`."
                    },
                    "num_estimators": { "type": "integer", "minimum": 1, "maximum": 64, "description": "Ensemble size. Default 8." },
                },
                "required": ["table", "target"],
            }),
        ),
        IMAGINE_TOOL => (
            IMAGINE_TOOL,
            "Generate one or more images from a prompt. The images are written to the run's \
             `generated/` folder and the result names each file's path and size; the bytes \
             never ride back as text. Read one with `vision` when you need to look at it.",
            json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "What to render." },
                    "aspect_ratio": { "type": "string", "description": "For example '1:1' or '16:9'. Default '1:1'." },
                    "n": { "type": "integer", "minimum": 1, "maximum": 4, "description": "How many images. Default 1." },
                    "provider": { "type": "string", "description": "Optional provider id to force." },
                },
                "required": ["prompt"],
            }),
        ),
        BROWSE_TOOL => (
            BROWSE_TOOL,
            "Drive a real browser. `open` a URL, `read` the current page's text (or one CSS \
             selector within it), or `screenshot` the current page to a PNG whose path the \
             result names. The browser persists across calls, so an `open` and the `read` \
             that follows see the same page.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["open", "read", "screenshot"],
                        "description": "What to do with the browser."
                    },
                    "url": { "type": "string", "description": "For `open`: the URL or file path to render." },
                    "wait_ms": { "type": "integer", "minimum": 0, "maximum": 10000, "description": "For `open`: milliseconds to wait before returning. Default 400." },
                    "selector": { "type": "string", "description": "For `read`: a CSS selector. Default 'body'." },
                    "path": { "type": "string", "description": "For `screenshot`: where to write the PNG. Default 'generated/page.png'." },
                    "width": { "type": "integer", "minimum": 200, "maximum": 4000, "description": "For `screenshot`: viewport width. Default 1280." },
                    "height": { "type": "integer", "minimum": 200, "maximum": 8000, "description": "For `screenshot`: viewport height. Default 1600." },
                },
                "required": ["action"],
            }),
        ),
        _ => return None,
    };
    Some(crate::api::ToolSpec::function(
        name,
        description,
        parameters,
    ))
}

/// Every capability tool's spec, in declaration order.
pub fn specs() -> Vec<crate::api::ToolSpec> {
    [MEASURE_TOOL, IMAGINE_TOOL, BROWSE_TOOL]
        .iter()
        .filter_map(|capability| spec(capability))
        .collect()
}

/// Whether `name` is a capability tool.
pub fn is_capability(name: &str) -> bool {
    matches!(name, MEASURE_TOOL | IMAGINE_TOOL | BROWSE_TOOL)
}

/// The validation subset the shared validator covers, plus the
/// capability's own conditional requirements: `open` needs a URL, and
/// the lane tools' schemas carry the rest.
pub fn validate(capability: &str, arguments: &Value) -> Result<(), String> {
    if arguments
        .get("prompt")
        .is_some_and(|prompt| prompt.as_str().unwrap_or("").trim().is_empty())
    {
        return Err("field `prompt` must not be empty".to_string());
    }
    match capability {
        BROWSE_TOOL => {
            let action = arguments
                .get("action")
                .and_then(|action| action.as_str())
                .unwrap_or("");
            match action {
                "open" => {
                    if arguments.get("url").and_then(|url| url.as_str()).is_none_or(|url| url.trim().is_empty()) {
                        return Err("field `url` is required when the action is `open`".to_string());
                    }
                }
                "read" | "screenshot" => {}
                other => {
                    return Err(format!(
                        "field `action` must be one of \"open\", \"read\", \"screenshot\"; got {other:?}"
                    ))
                }
            }
        }
        MEASURE_TOOL => {
            if arguments
                .get("table")
                .and_then(|table| table.as_array())
                .is_none()
            {
                return Err("field `table` expects an array; got a non-array".to_string());
            }
            if arguments
                .get("target")
                .and_then(|target| target.as_str())
                .is_none_or(str::is_empty)
            {
                return Err("field `target` is required and must be a non-empty string".to_string());
            }
        }
        IMAGINE_TOOL => {}
        other => return Err(format!("unknown capability `{other}`")),
    }
    Ok(())
}

/// Translate a capability call into the lane call the roster
/// dispatches: `(lane, lane tool, lane arguments)`.
pub fn translate(capability: &str, arguments: &Value) -> Option<(&'static str, String, Value)> {
    match capability {
        MEASURE_TOOL => {
            let mut lane = json!({
                "table": arguments.get("table")?,
                "target": arguments.get("target")?,
            });
            if let Some(task) = arguments.get("task_type") {
                lane["task_type"] = task.clone();
            }
            if let Some(estimators) = arguments.get("num_estimators") {
                lane["num_estimators"] = estimators.clone();
            }
            Some(("sdm1", "sdm1_classify".to_string(), lane))
        }
        IMAGINE_TOOL => {
            let mut lane = json!({ "prompt": arguments.get("prompt")? });
            if let Some(ratio) = arguments.get("aspect_ratio") {
                lane["aspect_ratio"] = ratio.clone();
            }
            if let Some(count) = arguments.get("n") {
                lane["n"] = count.clone();
            }
            if let Some(provider) = arguments.get("provider") {
                lane["provider"] = provider.clone();
            }
            Some(("gen1", "gen1_imagine".to_string(), lane))
        }
        BROWSE_TOOL => {
            let action = arguments.get("action")?.as_str()?;
            let (tool, lane) = match action {
                "open" => {
                    let mut lane = json!({ "url": arguments.get("url")? });
                    if let Some(wait) = arguments.get("wait_ms") {
                        lane["wait_ms"] = wait.clone();
                    }
                    ("browser_navigate", lane)
                }
                "read" => {
                    let mut lane = json!({});
                    if let Some(selector) = arguments.get("selector") {
                        lane["selector"] = selector.clone();
                    }
                    ("browser_read", lane)
                }
                "screenshot" => {
                    let mut lane = json!({
                        "path": arguments
                            .get("path")
                            .and_then(|path| path.as_str())
                            .unwrap_or("generated/page.png"),
                    });
                    if let Some(width) = arguments.get("width") {
                        lane["width"] = width.clone();
                    }
                    if let Some(height) = arguments.get("height") {
                        lane["height"] = height.clone();
                    }
                    ("browser_screenshot", lane)
                }
                _ => return None,
            };
            let _ = BROWSE_TOOL;
            Some(("browser", tool.to_string(), lane))
        }
        _ => None,
    }
}

/// Decode one data URI into `(mime, bytes)`, the shape `gen1_imagine`
/// answers with. `None` on a payload that is not `data:<mime>;base64,`.
pub fn decode_data_uri(uri: &str) -> Option<(String, Vec<u8>)> {
    let rest = uri.strip_prefix("data:")?;
    let (mime, encoded) = rest.split_once(";base64,")?;
    let bytes = crate::mcp::decode_base64(encoded)?;
    Some((mime.to_string(), bytes))
}

/// Write an imagine answer's images under `dir`, returning one
/// `(path, bytes, mime)` line per image. The model never sees base64.
pub fn write_images(dir: &Path, text: &str) -> Result<String, String> {
    let answer: Value = serde_json::from_str(text).map_err(|_| {
        "the image lane answered with text that is not the imagine shape".to_string()
    })?;
    let provider = answer
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let model = answer
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let images = answer
        .get("images")
        .and_then(Value::as_array)
        .ok_or_else(|| "the image lane answered with no images array".to_string())?;
    std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let mut lines = Vec::new();
    for (index, image) in images.iter().enumerate() {
        let uri = image
            .as_str()
            .ok_or_else(|| "an image entry is not a data URI".to_string())?;
        let (mime, bytes) = decode_data_uri(uri)
            .ok_or_else(|| "an image entry is not a decodable data URI".to_string())?;
        if bytes.len() > 20 * 1024 * 1024 {
            return Err(format!(
                "image {index} is {} bytes, past the 20 MiB cap",
                bytes.len()
            ));
        }
        let extension = match mime.as_str() {
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "png",
        };
        let path = dir.join(format!("image-{index}.{extension}"));
        std::fs::write(&path, &bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        lines.push(format!(
            "image {index}: {} ({} bytes, {mime}) \u{2014} provider {provider}, model {model}",
            path.display(),
            bytes.len()
        ));
    }
    if lines.is_empty() {
        return Err("the image lane answered with zero images".to_string());
    }
    Ok(lines.join("\n"))
}

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capability_declares_a_spec_and_maps_to_a_lane() {
        for capability in [MEASURE_TOOL, IMAGINE_TOOL, BROWSE_TOOL] {
            assert!(spec(capability).is_some(), "{capability} has no spec");
            let (lane, _) = lane_for(capability).expect("the lane");
            assert!(!lane.is_empty());
            assert!(is_capability(capability));
        }
        assert!(specs().len() == 3);
        assert!(!is_capability("bash"));
    }

    #[test]
    fn measure_translates_the_table_shape_and_validates_it() {
        let arguments = json!({
            "table": [{"x": 1, "target": 0}, {"x": 2}],
            "target": "target",
            "task_type": "regression",
        });
        assert!(validate(MEASURE_TOOL, &arguments).is_ok());
        let (lane, tool, lane_arguments) = translate(MEASURE_TOOL, &arguments).unwrap();
        assert_eq!(lane, "sdm1");
        assert_eq!(tool, "sdm1_classify");
        assert_eq!(lane_arguments["task_type"], "regression");
        assert_eq!(lane_arguments["target"], "target");
        // A missing table is a validation error, not a lane trip.
        assert!(validate(MEASURE_TOOL, &json!({"target": "x"})).is_err());
    }

    #[test]
    fn browse_requires_a_url_to_open_and_translates_three_actions() {
        assert!(validate(BROWSE_TOOL, &json!({"action": "open"})).is_err());
        assert!(validate(BROWSE_TOOL, &json!({"action": "click"})).is_err());
        let (lane, tool, lane_arguments) =
            translate(BROWSE_TOOL, &json!({"action": "open", "url": "http://x"})).unwrap();
        assert_eq!((lane, tool.as_str()), ("browser", "browser_navigate"));
        assert_eq!(lane_arguments["url"], "http://x");
        let (_, tool, _) = translate(BROWSE_TOOL, &json!({"action": "read"})).unwrap();
        assert_eq!(tool, "browser_read");
        let (_, tool, lane_arguments) =
            translate(BROWSE_TOOL, &json!({"action": "screenshot"})).unwrap();
        assert_eq!(tool, "browser_screenshot");
        assert_eq!(lane_arguments["path"], "generated/page.png");
    }

    #[test]
    fn an_imagine_answer_writes_files_and_names_them() {
        let dir = std::env::temp_dir().join(format!("fleet-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // A 1x1 red PNG.
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB0, 0x51, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let encoded = crate::mcp::encode_base64(png);
        let answer = json!({
            "provider": "qwen",
            "model": "qwen-image",
            "images": [format!("data:image/png;base64,{encoded}")],
        })
        .to_string();
        let result = write_images(&dir, &answer).unwrap();
        assert!(result.contains("provider qwen"), "{result}");
        let path = result
            .split(": ")
            .nth(1)
            .and_then(|rest| rest.split(" (").next())
            .expect("the path is named");
        assert!(std::path::Path::new(path).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_data_uri_decodes_to_its_bytes() {
        let (mime, bytes) = decode_data_uri("data:image/png;base64,AAEC").unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, vec![0x00, 0x01, 0x02]);
        assert!(decode_data_uri("http://not-a-data-uri").is_none());
    }
}
