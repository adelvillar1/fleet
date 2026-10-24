//! A token-budgeted map of a repository: the files worth knowing, ranked
//! by how much the rest of the tree refers to them, with their top-level
//! definitions.
//!
//! It is a **heuristic**, and it says so in its own output. There is no
//! parser here and no language server: a definition is a line matching a
//! per-language pattern at low indent, and a reference is an identifier
//! appearing in another file. That is enough to answer the question the
//! model actually has on arrival — *where does this repo keep the thing I
//! need to touch* — without a dependency it would then have to keep in
//! step with every grammar.
//!
//! Two properties matter more than accuracy:
//!
//! * **Deterministic.** The same tree returns the same text, byte for
//!   byte. Files sort by path, identifiers sort by name, scores break
//!   ties by path. A model that reads a map twice and gets two maps has
//!   learned that the map lies.
//! * **Budgeted.** The caller says how many tokens it will spend (default
//!   1,000) and the map stops at the line that would exceed it, saying so
//!   in the tail. Truncation is stated, never silent.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Roughly four bytes to a token, which is the conservative end for code.
const BYTES_PER_TOKEN: usize = 4;

/// The languages whose definitions this module recognises, by extension.
/// A file outside the set contributes references (its text still counts as
/// usage) but no definitions.
const LANGUAGES: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("py", "python"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("go", "go"),
    ("java", "java"),
    ("swift", "swift"),
    ("kt", "kotlin"),
    ("c", "c"),
    ("h", "c"),
    ("cc", "c"),
    ("cpp", "c"),
    ("hpp", "c"),
    ("rb", "ruby"),
    ("php", "php"),
];

/// One definition line: where it is and what it declares.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Definition {
    pub line: usize,
    pub text: String,
}

/// One file's entry in the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub language: Option<&'static str>,
    pub definitions: Vec<Definition>,
    /// How many times this file's identifiers appear in *other* files.
    pub references: usize,
}

impl FileEntry {
    /// The rank score: references first, then the file's own size as a
    /// tiebreaker, then the path. A file nothing refers to but that
    /// declares a great deal is still worth showing, behind the ones the
    /// tree leans on.
    fn score(&self) -> (usize, usize) {
        (self.references, self.definitions.len())
    }
}

/// A finished map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub text: String,
    /// Files the ranking saw, before the budget cut them.
    pub files: usize,
    /// Files whose entry survived the budget.
    pub shown: usize,
}

/// Build the map for `root`, spending at most `budget_tokens`.
pub fn build(root: &Path, budget_tokens: usize) -> Map {
    let (paths, walk_cut) = crate::search::tree_files(root);
    let mut entries: Vec<FileEntry> = Vec::new();
    // Identifier → the files that mention it, so reference counts are one
    // pass over the tree rather than one pass per file.
    let mut mentions: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut texts: Vec<String> = Vec::new();
    for path in &paths {
        let full = root.join(path);
        // A file too large to read in one go is not a file a map entry
        // helps with.
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        if text.len() > 1_000_000 {
            continue;
        }
        let shown = path.to_string_lossy().replace('\\', "/");
        let language = shown.rsplit_once('.').and_then(|(_, extension)| {
            LANGUAGES
                .iter()
                .find(|(suffix, _)| *suffix == extension)
                .map(|(_, language)| *language)
        });
        let definitions = language
            .map(|language| definitions_of(&text, language))
            .unwrap_or_default();
        for name in identifiers(&text) {
            mentions.entry(name).or_default().insert(texts.len());
        }
        texts.push(text);
        entries.push(FileEntry {
            path: shown,
            language,
            definitions,
            references: 0,
        });
    }
    // Count references: for each file's declared names, how many *other*
    // files mention them.
    for (index, entry) in entries.iter_mut().enumerate() {
        let mut total = 0usize;
        for definition in &entry.definitions {
            let Some(name) = declared_name(&definition.text) else {
                continue;
            };
            let Some(files) = mentions.get(name) else {
                continue;
            };
            total += files.iter().filter(|other| **other != index).count();
        }
        entry.references = total;
    }
    drop(texts);
    render(entries, budget_tokens, walk_cut)
}

/// Render ranked entries under a byte budget.
fn render(mut entries: Vec<FileEntry>, budget_tokens: usize, walk_cut: bool) -> Map {
    let files = entries.len();
    // Highest score first; the path breaks ties so the order never moves.
    entries.sort_by(|left, right| {
        right
            .score()
            .cmp(&left.score())
            .then_with(|| left.path.cmp(&right.path))
    });
    let budget = budget_tokens.max(1) * BYTES_PER_TOKEN;
    let mut text = String::from(
        "Repo map (heuristic: definitions by per-language pattern and a reference count, not a parser)\n",
    );
    let mut shown = 0usize;
    for entry in &entries {
        let mut block = String::new();
        block.push_str(&format!(
            "{}{}\n",
            entry.path,
            if entry.references > 0 {
                format!(" ({} refs)", entry.references)
            } else {
                String::new()
            }
        ));
        for definition in &entry.definitions {
            block.push_str(&format!("  {}: {}\n", definition.line, definition.text));
        }
        if text.len() + block.len() > budget {
            break;
        }
        text.push_str(&block);
        shown += 1;
    }
    if shown == 0 {
        text.push_str("(no file fit the token budget; raise budget_tokens)\n");
    }
    if shown < files {
        text.push_str(&format!(
            "({} of {files} file(s) shown, ranked by references; budget {budget_tokens} tokens{})\n",
            shown,
            if walk_cut { ", walk capped" } else { "" }
        ));
    } else if walk_cut {
        text.push_str("(the walk was capped; some files were not seen)\n");
    }
    Map { text, files, shown }
}

/// The definitions in `text` for `language`: lines whose first token
/// declares something, at most one indent level deep.
///
/// One level is the right line for both shapes this has to serve: an
/// `impl` block's members and a class's methods sit at one level and are
/// the definitions a map is for, while a function body's locals sit
/// deeper and are noise.
fn definitions_of(text: &str, language: &str) -> Vec<Definition> {
    let mut found = Vec::new();
    for (offset, line) in text.lines().enumerate() {
        let indent = line.len() - line.trim_start().len();
        if indent > 4 {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        if declares(trimmed, language) {
            let clipped: String = trimmed.chars().take(100).collect();
            found.push(Definition {
                line: offset + 1,
                text: clipped,
            });
            // A file that declares a thousand things is a generated file;
            // its first dozen is what a map can use.
            if found.len() >= 24 {
                break;
            }
        }
    }
    found
}

/// Does this line declare something in `language`?
fn declares(line: &str, language: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    let Some(first) = words.first() else {
        return false;
    };
    let prefixed = |prefix: &str| *first == prefix || line.starts_with(&format!("{prefix} "));
    match language {
        "rust" => {
            prefixed("fn")
                || prefixed("pub")
                || prefixed("struct")
                || prefixed("enum")
                || prefixed("trait")
                || prefixed("impl")
                || prefixed("mod")
                || prefixed("const")
                || prefixed("static")
                || prefixed("type")
                || prefixed("macro_rules!")
        }
        "python" => {
            prefixed("def") || prefixed("class") || (prefixed("async") && line.contains("def "))
        }
        "typescript" | "javascript" => {
            prefixed("function")
                || prefixed("class")
                || prefixed("interface")
                || prefixed("type")
                || prefixed("const")
                || prefixed("enum")
                || prefixed("export")
        }
        "go" => prefixed("func") || prefixed("type"),
        "java" | "kotlin" | "swift" => {
            prefixed("class")
                || prefixed("interface")
                || prefixed("enum")
                || prefixed("struct")
                || prefixed("fun")
                || prefixed("func")
                || prefixed("public")
                || prefixed("private")
                || prefixed("protocol")
        }
        "c" => {
            prefixed("struct")
                || prefixed("class")
                || prefixed("enum")
                || prefixed("#define")
                || prefixed("typedef")
        }
        "ruby" => prefixed("def") || prefixed("class") || prefixed("module"),
        "php" => prefixed("function") || prefixed("class") || prefixed("interface"),
        _ => false,
    }
}

/// The name a definition line declares, when one can be read off it. Used
/// to count references; a line whose name cannot be read contributes none.
fn declared_name(line: &str) -> Option<&str> {
    let line = line
        .trim_start_matches("pub ")
        .trim_start_matches("export ");
    let words: Vec<&str> = line.split_whitespace().collect();
    let index = words.iter().position(|word| {
        matches!(
            word.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_'),
            "fn" | "def"
                | "class"
                | "struct"
                | "enum"
                | "trait"
                | "interface"
                | "type"
                | "func"
                | "fun"
                | "module"
        )
    })?;
    let raw = words.get(index + 1)?;
    let name = raw
        .trim_start_matches('!')
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Every identifier of four characters or more in `text`. Short names are
/// noise: `new`, `len`, `get` say nothing about where a file sits.
fn identifiers(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            if current.len() >= 4 && !current.chars().all(|c| c.is_ascii_digit()) {
                found.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
    }
    if current.len() >= 4 {
        found.push(current);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("fleet-repo-map-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, text).unwrap();
        }
        root
    }

    #[test]
    fn the_map_is_deterministic_and_names_its_heuristic() {
        let root = tree(
            "deterministic",
            &[
                ("src/alpha.rs", "pub fn alpha_call() {}\n"),
                ("src/beta.rs", "fn beta_call() { alpha_call(); }\n"),
                (
                    "src/gamma.rs",
                    "fn gamma_call() { beta_call(); alpha_call(); }\n",
                ),
            ],
        );
        let first = build(&root, 1_000);
        let second = build(&root, 1_000);
        assert_eq!(first.text, second.text);
        assert!(
            first.text.starts_with("Repo map (heuristic"),
            "{}",
            first.text
        );
        assert!(first.text.contains("alpha_call"), "{}", first.text);
        // alpha is referenced by the other two, so it ranks first.
        let alpha = first.text.find("src/alpha.rs").unwrap();
        let gamma = first.text.find("src/gamma.rs").unwrap();
        assert!(alpha < gamma, "{}", first.text);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_budget_cuts_and_says_so() {
        let mut files: Vec<(String, String)> = Vec::new();
        for index in 0..40 {
            files.push((
                format!("src/file{index}.rs"),
                format!("pub fn name{index}() {{}}\n"),
            ));
        }
        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let root = tree("budget", &borrowed);
        let small = build(&root, 60);
        assert!(small.shown < small.files, "{}", small.text);
        assert!(small.text.contains("of 40 file(s) shown"), "{}", small.text);
        assert!(
            small.text.len() <= 60 * BYTES_PER_TOKEN + 200,
            "{} bytes",
            small.text.len()
        );
        let wide = build(&root, 10_000);
        assert_eq!(wide.shown, wide.files);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_empty_tree_says_it_has_nothing() {
        let root =
            std::env::temp_dir().join(format!("fleet-repo-map-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let map = build(&root, 1_000);
        assert!(map.text.contains("no file fit"), "{}", map.text);
        assert_eq!(map.shown, 0);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn definitions_are_read_per_language() {
        // One level in is mapped (an impl's member, a class's method);
        // two levels in is a function body and is not.
        let rust = definitions_of(
            "use x;\npub fn one() {}\n    fn nested() {}\n        fn inner() {}\n",
            "rust",
        );
        assert_eq!(rust.len(), 2, "{rust:?}");
        assert_eq!(rust[0].line, 2);
        assert_eq!(rust[1].line, 3);
        let python = definitions_of("class Thing:\n    def method(self):\n", "python");
        assert_eq!(python.len(), 2, "{python:?}");
        let long = definitions_of(&"fn many() {}\n".repeat(40), "rust");
        assert_eq!(long.len(), 24);
    }
}
