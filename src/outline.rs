//! Structural source outlines — the retrievable unit for code.
//!
//! grep finds lines and `repo_map` pattern-matches definitions; neither gives
//! the retriever a symbol's shape. This module parses source with tree-sitter
//! and emits one outline per file: its symbols with their signatures, members
//! and calls, capped. The outline's own text is what the lexical index stores,
//! so a query naming a symbol's signature retrieves the symbol rather than a
//! line that happens to mention it.
//!
//! The grammar set is bounded to the languages the workspace's own code and
//! docs are written in. The grammars ship in their crates and are compiled by
//! the build — hard rule 12's "built by the install", never supplied by a
//! checkout. TOML is deliberately absent: the only TOML grammar targets the
//! legacy tree-sitter API and would force a second, incompatible runtime.

use tree_sitter::{Node, Parser, Tree};

/// The most calls one symbol records.
pub const MAX_CALLS: usize = 24;
/// The most members one symbol records.
pub const MAX_MEMBERS: usize = 32;
/// The most characters of one line in a signature.
pub const MAX_LINE_CHARS: usize = 180;
/// The most characters in one outline's retrievable text.
pub const MAX_OUTLINE_CHARS: usize = 3_600;

/// The languages the outline extractor parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineLanguage {
    Rust,
    Python,
    Markdown,
    TypeScript,
    Tsx,
    Json,
    Bash,
}

impl OutlineLanguage {
    /// The language for a path, by its extension. `None` when the path's
    /// language is outside the bounded set — an outline is not attempted
    /// rather than guessed.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match extension.as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" => Self::Python,
            "md" | "markdown" => Self::Markdown,
            "ts" => Self::TypeScript,
            "tsx" | "jsx" => Self::Tsx,
            "json" => Self::Json,
            "sh" | "bash" | "zsh" => Self::Bash,
            _ => return None,
        })
    }

    fn language(&self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Markdown => tree_sitter_md::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }

    /// The node kinds that name a definition, per language. The list is
    /// deliberately explicit: an outline is the file's structure, not every
    /// named node in it.
    fn definition_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &[
                "function_item",
                "struct_item",
                "enum_item",
                "trait_item",
                "impl_item",
                "mod_item",
                "const_item",
                "static_item",
                "type_item",
                "union_item",
                "macro_definition",
            ],
            Self::Python => &["function_definition", "class_definition"],
            Self::Markdown => &["atx_heading", "setext_heading"],
            Self::TypeScript | Self::Tsx => &[
                "function_declaration",
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
                "type_alias_declaration",
                "method_definition",
                "lexical_declaration",
                "variable_declaration",
            ],
            Self::Json => &["pair"],
            Self::Bash => &["function_definition"],
        }
    }

    fn call_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["call_expression", "macro_invocation"],
            Self::Python => &["call"],
            Self::TypeScript | Self::Tsx => &["call_expression", "new_expression"],
            Self::Bash => &["command"],
            Self::Json | Self::Markdown => &[],
        }
    }

    fn member_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["field_declaration", "const_item", "type_item"],
            Self::Python => &[
                "expression_statement",
                "function_definition",
                "class_definition",
            ],
            Self::TypeScript | Self::Tsx => &[
                "property_definition",
                "method_definition",
                "public_field_definition",
            ],
            Self::Json => &["pair"],
            Self::Bash | Self::Markdown => &[],
        }
    }
}

/// One symbol in an outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineSymbol {
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub members: Vec<String>,
    pub calls: Vec<String>,
    /// The symbol's byte offset in the source, so the outline reads top to
    /// bottom whatever order the tree walk found them in.
    pub position: usize,
}

impl OutlineSymbol {
    /// The symbol as it enters the retrievable text: the signature, then the
    /// members, then the calls — the shape a query can name.
    fn as_text(&self) -> String {
        let mut text = self.signature.clone();
        if !self.members.is_empty() {
            text.push_str("\n  members: ");
            text.push_str(&self.members.join(", "));
        }
        if !self.calls.is_empty() {
            text.push_str("\n  calls: ");
            text.push_str(&self.calls.join(", "));
        }
        text
    }
}

/// The outline of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outline {
    pub path: String,
    pub language: Option<OutlineLanguage>,
    pub symbols: Vec<OutlineSymbol>,
    pub headings: Vec<String>,
    /// The outline's retrievable text, capped.
    pub text: String,
}

/// Extract the outline of a source file. A path outside the bounded language
/// set yields an outline with no symbols and no text — the caller decides
/// whether to index it.
pub fn extract(path: &str, source: &str) -> Outline {
    let language = OutlineLanguage::from_path(std::path::Path::new(path));
    let mut outline = Outline {
        path: path.to_string(),
        language,
        ..Outline::default()
    };
    let Some(language) = language else {
        return outline;
    };
    let Some(tree) = parse(language, source) else {
        return outline;
    };
    let root = tree.root_node();
    match language {
        OutlineLanguage::Markdown => collect_headings(root, source, &mut outline),
        _ => collect_symbols(root, source, language, &mut outline),
    }
    outline.text = cap(&outline_text(&outline), MAX_OUTLINE_CHARS);
    outline
}

fn parse(language: OutlineLanguage, source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language.language()).ok()?;
    parser.parse(source, None)
}

fn collect_symbols(root: Node, source: &str, language: OutlineLanguage, outline: &mut Outline) {
    let definitions = language.definition_kinds();
    let calls = language.call_kinds();
    let members = language.member_kinds();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if definitions.contains(&node.kind()) {
            outline.symbols.push(symbol(node, source, calls, members));
        }
        let mut child = node.walk();
        for (index, kid) in node.children(&mut child).enumerate() {
            let _ = index;
            stack.push(kid);
        }
    }
    // The tree walk is depth-first from the root, so the symbols come out in
    // source order only after a sort by position — an outline reads top to
    // bottom.
    outline.symbols.sort_by_key(|symbol| symbol.position);
}

fn symbol(node: Node, source: &str, calls: &[&str], members: &[&str]) -> OutlineSymbol {
    let name = node
        .child_by_field_name("name")
        .map(|name| text(name, source))
        .unwrap_or_else(|| head(text(node, source)));
    let signature = first_line(text(node, source));
    OutlineSymbol {
        kind: node.kind().to_string(),
        name,
        signature,
        members: collect_members(node, source, members),
        calls: collect_calls(node, source, calls),
        position: node.start_byte(),
    }
}

fn collect_members(node: Node, source: &str, members: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    let Some(body) = node.child_by_field_name("body") else {
        return found;
    };
    let mut stack = vec![body];
    while let Some(current) = stack.pop() {
        if members.contains(&current.kind()) {
            let label = current
                .child_by_field_name("name")
                .map(|name| text(name, source))
                .unwrap_or_else(|| first_line(text(current, source)));
            if !label.is_empty() {
                found.push(label);
            }
            if found.len() >= MAX_MEMBERS {
                break;
            }
        }
        let mut child = current.walk();
        for kid in current.children(&mut child) {
            stack.push(kid);
        }
    }
    found
}

fn collect_calls(node: Node, source: &str, calls: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if calls.contains(&current.kind()) {
            let callee = current
                .child_by_field_name("function")
                .or_else(|| current.child_by_field_name("name"))
                .map(|callee| first_line(text(callee, source)))
                .unwrap_or_default();
            if !callee.is_empty() && !found.contains(&callee) {
                found.push(callee);
            }
            if found.len() >= MAX_CALLS {
                break;
            }
        }
        let mut child = current.walk();
        for kid in current.children(&mut child) {
            stack.push(kid);
        }
    }
    found
}

fn collect_headings(root: Node, source: &str, outline: &mut Outline) {
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "atx_heading" | "setext_heading") {
            let text = node
                .child_by_field_name("heading_content")
                .map(|content| first_line(text(content, source)))
                .unwrap_or_default();
            if !text.is_empty() {
                found.push((node.start_byte(), text));
            }
        }
        let mut child = node.walk();
        for kid in node.children(&mut child) {
            stack.push(kid);
        }
    }
    found.sort_by_key(|(position, _)| *position);
    outline.headings = found.into_iter().map(|(_, text)| text).collect();
}

fn outline_text(outline: &Outline) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !outline.headings.is_empty() {
        parts.push(outline.headings.join("\n"));
    }
    for symbol in &outline.symbols {
        parts.push(symbol.as_text());
    }
    parts.join("\n")
}

fn text(node: Node, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or_default()
        .to_string()
}

fn first_line(text: String) -> String {
    let line = text.lines().next().unwrap_or("");
    cap(line, MAX_LINE_CHARS)
}

fn head(text: String) -> String {
    let trimmed = text.trim();
    let head: String = trimmed.chars().take(60).collect();
    head
}

fn cap(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rust_file_yields_its_symbols_with_signatures_members_and_calls() {
        let source = r#"
pub struct Engine {
    pub name: String,
    pub dims: usize,
}

impl Engine {
    pub fn open(&self, path: &str) -> Result<Store, String> {
        let store = load(path);
        store.validate()
    }
}

pub fn rank(query: &str, store: &Store) -> Vec<Hit> {
    store.search(query)
}
"#;
        let outline = extract("src/engine.rs", source);
        assert_eq!(outline.language, Some(OutlineLanguage::Rust));
        let names: Vec<&str> = outline
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect();
        assert!(
            names.contains(&"Engine"),
            "the struct is outlined: {names:?}"
        );
        assert!(names.contains(&"open"), "the method is outlined: {names:?}");
        assert!(
            names.contains(&"rank"),
            "the function is outlined: {names:?}"
        );
        let open = outline
            .symbols
            .iter()
            .find(|symbol| symbol.name == "open")
            .unwrap();
        assert!(
            open.signature.contains("pub fn open"),
            "the signature carries the shape: {:?}",
            open.signature
        );
        assert!(
            open.calls.contains(&"load".to_string()),
            "the calls are recorded: {:?}",
            open.calls
        );
        let engine = outline
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Engine")
            .unwrap();
        assert!(
            engine.members.contains(&"name".to_string()),
            "the members are recorded: {:?}",
            engine.members
        );
    }

    #[test]
    fn a_python_file_yields_its_definitions() {
        let outline = extract(
            "app/main.py",
            "def resolve(query, store):\n    return store.search(query)\n\nclass Engine:\n    def open(self):\n        pass\n",
        );
        let names: Vec<&str> = outline
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect();
        assert!(names.contains(&"resolve"), "{names:?}");
        assert!(names.contains(&"Engine"), "{names:?}");
        assert!(names.contains(&"open"), "{names:?}");
    }

    #[test]
    fn markdown_yields_its_headings_and_no_symbols() {
        let outline = extract("doc.md", "# Title\n\n## Section\n\nbody\n");
        assert_eq!(outline.headings, vec!["Title", "Section"]);
        assert!(outline.symbols.is_empty(), "headings are not symbols");
    }

    #[test]
    fn a_path_outside_the_set_yields_an_empty_outline() {
        let outline = extract("image.png", "binary");
        assert_eq!(outline.language, None);
        assert!(outline.symbols.is_empty());
        assert!(outline.text.is_empty());
    }

    #[test]
    fn the_caps_hold() {
        let calls: Vec<String> = (0..40).map(|index| format!("call_{index}")).collect();
        let long = format!("fn big() {{ {} }}", calls.join("; "));
        let outline = extract("src/big.rs", &long);
        let big = outline
            .symbols
            .iter()
            .find(|symbol| symbol.name == "big")
            .expect("the function is outlined");
        assert!(
            big.calls.len() <= MAX_CALLS,
            "the calls are capped at {MAX_CALLS}"
        );
        assert!(
            outline.text.chars().count() <= MAX_OUTLINE_CHARS,
            "the outline text is capped at {MAX_OUTLINE_CHARS}"
        );
    }

    #[test]
    fn the_outline_text_carries_the_signature_for_a_query() {
        let outline = extract(
            "src/engine.rs",
            "pub fn resolve_query(query: &str) -> Vec<Hit> { todo!() }\n",
        );
        assert!(
            outline.text.contains("resolve_query"),
            "the retrievable text carries the symbol's signature: {:?}",
            outline.text
        );
    }
}
