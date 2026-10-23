//! The approval gate's three modes: `ask`, `yes`, and `yes-guarded`.
//!
//! ## Why a third mode exists
//!
//! `fleet run --yes` approves every tool call, and for an owner sitting
//! at the keyboard that is the honest reading of "I already decided."
//! The owner's house rule (CLAUDE.md, hard rule 7) is narrower than
//! that: a destructive operation requires explicit approval **in the
//! current turn**. So a mode that approves `rm -rf` as readily as
//! `grep` waives the owner's rule on the owner's behalf — silently, and
//! for the rest of the run. `yes-guarded` is the middle setting that
//! does not: it approves everything an unattended run actually needs
//! and still stops on the destructive class. It is the owner's house
//! rule expressed as a machine-checkable class, which is what lets the
//! rule survive the owner being AFK.
//!
//! ## The class is named, not guessed
//!
//! [`DESTRUCTIVE`] is a short list of substrings, written down, that a
//! reviewer reads in one pass. It is deliberately not a heuristic. A
//! parser that infers intent from a shell string guesses wrong in both
//! directions: it waves through a destructive command spelled unusually
//! and it stops an ordinary one that happens to mention a scary word. A
//! short named list that misses is a gap the owner can see and close in
//! one edit; a clever classifier that guesses wrong is a gate that lies
//! about what it checked. The bias that remains is deliberate and
//! over-inclusive — `grep -r "rm -rf" .` is stopped too, and that is
//! the cheap error.
//!
//! ## Resolution lives here
//!
//! [`resolve`] decides the mode from the CLI flag and the config word,
//! in one place, so every surface — `run`, `chat`, `cron`, `doctor` —
//! reads the same order and a second wiring site cannot drift from the
//! first.
//!
//! ## The name `Gate`
//!
//! This is the approvals gate. It is not [`crate::improve::Gate`], the
//! local test gate a proposal survives. Both answer "may this happen?";
//! neither is the other, and the module path is the disambiguation.

use crate::api::ToolCall;

/// How a run answers "may this tool call execute?".
///
/// [`Mode::Ask`] is the default and the only mode that ever prompts.
/// [`Mode::Yes`] is the run-scoped latch `--yes` buys. [`Mode::YesGuarded`]
/// is the unattended middle: everything but the destructive class runs
/// without a prompt, and the destructive class still asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Prompt for every call; the console or the keyboard answers.
    Ask,
    /// Approve every call, destructive ones included. This is what
    /// `--yes` has always meant, and it stays available on purpose —
    /// an owner who wants the class back sets `yes-guarded` in the
    /// config rather than losing the unguarded latch.
    Yes,
    /// Approve every call except the destructive class, which still
    /// asks. The unattended setting that keeps hard rule 7 intact.
    YesGuarded,
}

impl Mode {
    /// The word the surface shows: `ask`, `yes`, `yes-guarded`.
    ///
    /// These three words are the config vocabulary as well, so a status
    /// line, a `doctor` row, and a session-log record all name a mode
    /// the owner can find in the setting that produced it.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Ask => "ask",
            Mode::Yes => "yes",
            Mode::YesGuarded => "yes-guarded",
        }
    }
}

impl Default for Mode {
    /// A run that names nothing asks. The default is the safe branch
    /// because every other value is a decision the owner has to have
    /// made somewhere a reader can see.
    fn default() -> Self {
        Mode::Ask
    }
}

/// Resolve the mode from the CLI flag and the config word, in the one
/// order every surface uses.
///
/// `explicit` is `Some(true)` when the owner passed `--yes` on the
/// command line, `Some(false)` when the flag was absent, and `None`
/// when the caller has no flag to report at all. `config` is the
/// configured word, whatever carries it.
///
/// The order:
///
/// 1. An explicit `--yes` is [`Mode::Yes`]. The flag never *narrows*
///    what a narrower config says — `--yes` on a `yes-guarded` config is
///    `Yes`, because the owner typed it this turn and a flag is the
///    opposite of silent — and it is never widened into something else
///    by the file either: the decision the owner made on the command
///    line is the decision the run gets.
/// 2. Otherwise the config word decides: `yes` → [`Mode::Yes`],
///    `yes-guarded` → [`Mode::YesGuarded`], and `ask`, an absent word,
///    or any word this function does not know → [`Mode::Ask`].
///
/// The unknown word reading as `Ask` is the fail-closed branch, named
/// here so nobody has to infer it: an unrecognized setting must not
/// quietly become an unattended run. The word is matched exactly, after
/// trimming surrounding whitespace — a stray newline from a config file
/// is a formatting accident, while a different case or a misspelling is
/// a word this function does not know and therefore reads as `Ask`.
pub fn resolve(explicit: Option<bool>, config: Option<&str>) -> Mode {
    if explicit == Some(true) {
        return Mode::Yes;
    }
    match config.map(str::trim) {
        Some("yes") => Mode::Yes,
        Some("yes-guarded") => Mode::YesGuarded,
        // `ask`, absent, or a word this function does not know.
        _ => Mode::Ask,
    }
}

/// The substrings that put a `bash` command in the destructive class.
///
/// This is the one place the class is named — the plan's C2, and the
/// reason `yes-guarded` is more than a config flag. Every entry marks a
/// shape the owner's house rule covers:
///
/// * recursive removal: `rm -rf`, and `rm -r ` with its trailing space,
///   which is what keeps the two from matching each other's commands;
/// * the git remote and working-tree rewrites: `git push --force`,
///   `git push -f`, `git reset --hard`, `git clean -`;
/// * history rewrite proper: `git filter-branch`. `git rebase -i` is
///   deliberately **not** here — an interactive rebase is a review
///   surface, not a rewrite by itself, and listing it would train the
///   owner to ignore the prompt;
/// * raw disk writes: `dd if=`, `mkfs`, `> /dev/sd`;
/// * a permissions wipe: `chmod -R 000`;
/// * and any command that names a credential file: `/.env`, `id_rsa`,
///   `.pem`, `credentials`.
///
/// A command matching none of these runs unattended under
/// [`Mode::YesGuarded`]. A command matching any of them stops and asks,
/// however ordinary the rest of the line looks.
pub const DESTRUCTIVE: &[&str] = &[
    "rm -rf",
    "rm -r ",
    "git push --force",
    "git push -f",
    "git reset --hard",
    "git clean -",
    "git filter-branch",
    "dd if=",
    "mkfs",
    "> /dev/sd",
    "chmod -R 000",
    "/.env",
    "id_rsa",
    ".pem",
    "credentials",
];

/// The bash tool's name, as the API declares it.
///
/// Only this tool's arguments carry a `command`, and only a shell has
/// the reach the destructive class is about — every other tool in the
/// set is out of the class by construction rather than by inspection.
const BASH_TOOL: &str = "bash";

/// The call as the gate reads it, whatever name it arrived under: the
/// `subagent → ` label — one per delegation level — is stripped, and a
/// labelled call's wrapper arguments (`goal`, `tool`, `arguments`) are
/// unwrapped to the tool call the child actually issued. The class
/// rules key on what a call does, not on where it came from, so the
/// destructive check and the always-ask check read the bare call at
/// every depth (KI-209, KI-210). A call with no label passes through
/// byte-for-byte.
fn unlabel(call: &ToolCall) -> ToolCall {
    let prefix = crate::tools::subagent_call_prefix();
    let mut name = call.function.name.as_str();
    let mut labelled = false;
    while let Some(stripped) = name.strip_prefix(&prefix) {
        name = stripped;
        labelled = true;
    }
    let mut arguments = call.function.arguments.clone();
    if labelled {
        if let Some(inner) = serde_json::from_str::<serde_json::Value>(&arguments)
            .ok()
            .and_then(|value| {
                value
                    .get("arguments")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
        {
            arguments = inner;
        }
    }
    ToolCall {
        id: call.id.clone(),
        kind: call.kind.clone(),
        function: crate::api::FunctionCall {
            name: name.to_string(),
            arguments,
        },
    }
}

/// The command a call would run, or `None` when there is no command to
/// read: the call is not a `bash` call, its arguments are not valid
/// JSON, or the arguments name no `command` string. A forwarded child
/// call is read through its label (KI-209).
///
/// The `None` case is not "safe", it is "unknown" — [`Gate::is_destructive`]
/// decides which of the two a caller should treat it as, and it treats
/// unknown as destructive.
fn bash_command(call: &ToolCall) -> Option<String> {
    let call = &unlabel(call);
    if call.function.name != BASH_TOOL {
        return None;
    }
    let arguments: serde_json::Value = serde_json::from_str(&call.function.arguments).ok()?;
    arguments.get("command")?.as_str().map(str::to_string)
}

/// The gate a run consults before each call.
///
/// The gate answers one question — may this call run without asking? —
/// and answers it from the mode alone. It never prompts, never reads
/// input, and never blocks: prompting is [`crate::agent::ConsoleApprover`]'s
/// or the TUI's job, and a gate that tried to do both would make the
/// unattended path and the interactive one impossible to test apart.
#[derive(Debug, Clone)]
pub struct Gate {
    mode: Mode,
    /// Calls a server itself marked destructive. A hint forces the
    /// prompt path even under a mode that would approve the call; it
    /// never makes a call approvable.
    hinted: std::collections::BTreeSet<String>,
}

impl Gate {
    /// A gate in `mode`.
    pub fn new(mode: Mode) -> Gate {
        Gate {
            mode,
            hinted: std::collections::BTreeSet::new(),
        }
    }

    /// The mode this gate enforces.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Whether the mode lets this call through without a prompt.
    ///
    /// [`Mode::Ask`] approves nothing — that is the whole of its
    /// meaning. [`Mode::Yes`] approves everything. [`Mode::YesGuarded`]
    /// approves everything except the destructive class, which falls
    /// through to the ordinary prompt rather than to a denial: the
    /// owner is AFK, not absent, and the run resumes the moment they
    /// answer.
    pub fn approves_without_asking(&self, call: &ToolCall) -> bool {
        // A server's destructive hint sits with the always-ask class:
        // the hint came from the server, so it can only raise the bar.
        if self.hinted.contains(&call.id) {
            return false;
        }
        // The always-ask class sits ABOVE the modes: a call in it asks
        // even under `yes`, because its blast radius is the machine
        // itself and no execution mode waives that.
        if asks_regardless(call) {
            return false;
        }
        match self.mode {
            Mode::Ask => false,
            Mode::Yes => true,
            Mode::YesGuarded => !Gate::is_destructive(call),
        }
    }

    /// Whether this call is in the destructive class, independent of the
    /// mode.
    ///
    /// Public because the surface has to be able to name *why* it is
    /// still asking: a prompt that says "yes-guarded stopped here"
    /// without saying which class the call fell into is a prompt the
    /// owner has to re-derive, and the answer is already computed here.
    ///
    /// Non-bash tools are never destructive — there is no command to
    /// match, so there is nothing to over-match either. A `bash` call
    /// whose arguments cannot be read into a command string **is**
    /// destructive: an argument blob we cannot parse is a command we
    /// cannot vouch for, and the fail-closed reading is the only one
    /// that does not require the gate to guess.
    pub fn is_destructive(call: &ToolCall) -> bool {
        let Some(command) = bash_command(call) else {
            return call.function.name == BASH_TOOL;
        };
        DESTRUCTIVE.iter().any(|mark| command.contains(mark))
    }

    /// The sentence a guarded denial carries, model-visible and logged.
    ///
    /// It mirrors [`crate::agent::Approver::denial_reason`]: the
    /// ordinary denial names a user who was asked and said no, and this
    /// one has no user to name — the run stopped on its own policy — so
    /// it names the policy instead. An owner reading a log after the
    /// fact has to answer "why did this run stop?" without re-deriving
    /// the mode, so the sentence carries the mode's own label, the fact
    /// that the call fell into the destructive class, and the command
    /// that put it there.
    pub fn denial_text(&self, call: &ToolCall) -> String {
        let subject = match bash_command(call) {
            Some(command) => format!("`{command}`"),
            None => format!(
                "a `{}` call whose arguments could not be read",
                call.function.name
            ),
        };
        format!(
            "denied: the mode is {} and this call is in the destructive class — {} — \
             which the owner's house rule requires explicit approval for in the current turn; \
             the run stops here rather than waiving the rule on the owner's behalf.",
            self.mode.label(),
            subject
        )
    }
}

impl crate::agent::Approver for Gate {
    /// The mode decides whether a call runs, and a guarded denial
    /// carries its own sentence: the owner's house rule is that
    /// destructive operations need approval in the current turn, so a
    /// run that stops on one must say WHICH rule stopped it rather
    /// than naming a user who is not there.
    fn approve(&mut self, call: &crate::api::ToolCall) -> bool {
        self.approves_without_asking(call)
    }

    /// A destructive hint from the server forces the prompt path: the
    /// gate records the call and its auto-approval refuses it. A hint
    /// never makes a call approvable, so this method only ever adds to
    /// the set.
    fn annotate(&mut self, call: &crate::api::ToolCall, destructive: bool) {
        if destructive {
            self.hinted.insert(call.id.clone());
        }
    }

    /// The denial's sentence, or `None` for the modes that never deny
    /// on their own. The trait takes no call — the reason is the run's,
    /// not one call's so the gate names the class it holds rather than
    /// the command it just refused.
    ///
    /// Two classes deny here and both must be named honestly: the
    /// destructive class under `yes-guarded`, and the always-ask class
    /// under ANY mode. The second matters more — a run in `yes` that
    /// stops on a desktop action must not report "the user did not
    /// approve this call", because there was no user and the mode said
    /// yes. The rule denied it, and the model has to be able to tell
    /// the owner that.
    fn denial_reason(&self) -> Option<String> {
        if !self.hinted.is_empty() {
            return Some(
                "denied: the server marks this tool destructive, which the owner's rule keeps                  behind a prompt; the run stops here rather than trusting a hint from the                  server as permission."
                    .to_string(),
            );
        }
        if self.mode == Mode::YesGuarded {
            return Some(format!(
                "approvals are `{}` and this call is in the destructive class \
                 ({}), which the owner's rule keeps behind a prompt",
                Mode::YesGuarded.label(),
                DESTRUCTIVE.len()
            ));
        }
        // `yes` (or a config of `yes`) with an always-ask tool in the
        // declaration: the class that no mode waives.
        (self.mode == Mode::Yes).then(|| {
            format!(
                "approvals are `{}` and this call is in the always-ask class \
                 ({}), which no execution mode waives — the owner's ruling \
                 for tools whose blast radius is the machine itself",
                Mode::Yes.label(),
                ALWAYS_ASK_TOOLS.join(", ")
            )
        })
    }
}

/// The approval word from `<FLEET_HOME>/config.json`, read the way the
/// roster reads its own: a missing file is the default, a malformed one
/// is ignored rather than fatal, and an unknown word is `Ask` — the
/// fail-closed reading `resolve` already applies.
pub fn config_word(home: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(home.join("config.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("approvals")
        .and_then(|word| word.as_str())
        .map(|word| word.to_string())
}

/// The tools that ask in EVERY mode — `yes` included.
///
/// Computer use through the cua driver drives the real desktop: a
/// click, a keystroke, a screenshot of whatever is on screen. It is the
/// one tool whose blast radius is the machine itself rather than a file
/// or a process, so no execution mode waives it. The owner's ruling
/// (2026-10-15): it requests approval every time regardless of the
/// mode we are on.
///
/// The list names the tool before the tool exists, deliberately: the
/// ruling is encoded here so the tool inherits it when it lands,
/// rather than needing a second change to remember. An empty-name entry
/// would be a bug, so the test pins that every entry is a real tool
/// name — and `computer_use` is not yet one, which the test says
/// plainly rather than skipping.
pub const ALWAYS_ASK_TOOLS: &[&str] = &["computer_use"];

/// Whether this call asks regardless of mode: a tool in
/// [`ALWAYS_ASK_TOOLS`], or a forwarded child call to one. A subagent's
/// calls reach the owner's gate named `subagent → <tool>` — one label
/// per delegation level — and the prefix is what the prompt reads, not
/// what the rule keys on, so every repetition is stripped before the
/// lookup (KI-210: the class survives depth 2).
pub fn asks_regardless(call: &ToolCall) -> bool {
    ALWAYS_ASK_TOOLS.contains(&unlabel(call).function.name.as_str())
}

/// One remembered approval, kept so its class stops prompting.
///
/// A rule names a tool and, for the shell, the program that leads the
/// command: `{"tool":"bash","program":"cargo"}` remembers every `cargo`
/// invocation without remembering anything about its flags. The program
/// is the part of a command that stays the same between two calls to
/// the same tool and the arguments are the part that does not, so the
/// rule is keyed where it stays true. A rule for any other tool is the
/// tool name alone — a `write` call IS its arguments, and a rule that
/// tried to narrow further would be remembering one call rather than a
/// class of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The tool's name, as the API declares it, with any `subagent → `
    /// label stripped: the class rules key on what a call does, not on
    /// where it came from (KI-209, KI-210).
    pub tool: String,
    /// The leading program of the command, for the shell only. `None`
    /// for every other tool — and for a hand-written `{"tool":"bash"}`
    /// entry, which reads as every shell call.
    pub program: Option<String>,
}

impl Rule {
    /// The rule a "don't ask again" on this call would write.
    ///
    /// Derived from the call here, in the module that already reads a
    /// call's tool and command for the class checks, so there is one
    /// place a call becomes a rule and a second site cannot derive a
    /// different one.
    pub fn for_call(call: &ToolCall) -> Rule {
        let bare = unlabel(call);
        let program = bash_command(call)
            .as_deref()
            .and_then(|command| command.split_whitespace().next())
            .map(str::to_string);
        Rule {
            tool: bare.function.name,
            program,
        }
    }

    /// The rule as config.json carries it. `program` is omitted when
    /// there is none, so the file reads the way the class is spoken.
    pub fn to_value(&self) -> serde_json::Value {
        let mut object = serde_json::Map::new();
        object.insert(
            "tool".to_string(),
            serde_json::Value::String(self.tool.clone()),
        );
        if let Some(program) = &self.program {
            object.insert(
                "program".to_string(),
                serde_json::Value::String(program.clone()),
            );
        }
        serde_json::Value::Object(object)
    }

    /// The rule one list entry describes, or `None` when the entry is
    /// not a rule: not an object, naming no tool, or carrying a program
    /// that is not a string. A caller reading a hand-edited file skips
    /// such an entry rather than treating it as a rule about nothing.
    pub fn from_value(value: &serde_json::Value) -> Option<Rule> {
        let object = value.as_object()?;
        let tool = object.get("tool")?.as_str()?.trim().to_string();
        if tool.is_empty() {
            return None;
        }
        let program = match object.get("program") {
            Some(program) => Some(program.as_str()?.trim().to_string()),
            None => None,
        };
        Some(Rule { tool, program })
    }

    /// The rule as one line: `bash · cargo`, `write`.
    pub fn label(&self) -> String {
        match &self.program {
            Some(program) => format!("{} · {}", self.tool, program),
            None => self.tool.clone(),
        }
    }

    /// Whether this rule covers `call`.
    ///
    /// The tool must match exactly, after the label is stripped. A rule
    /// with a program matches a shell call whose command leads with
    /// that program; a rule without one matches every call to the tool,
    /// which is what a hand-written `{"tool":"bash"}` asks for.
    fn covers_call(&self, call: &ToolCall) -> bool {
        if unlabel(call).function.name != self.tool {
            return false;
        }
        let Some(program) = &self.program else {
            return true;
        };
        bash_command(call)
            .as_deref()
            .and_then(|command| command.split_whitespace().next())
            == Some(program.as_str())
    }
}

/// The remembered approvals a surface consults before it prompts.
///
/// This is the second half of the seat: the modes say whether a run
/// asks at all, and these say which of the calls it asks about the
/// owner has already answered. The two compose in the GUI's approver —
/// remembered rules first, then the mode's own gate, then the prompt —
/// so a run in `ask` with one remembered rule prompts for everything
/// except that rule's class, which is what "prompts thin out over
/// time" means concretely.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    /// Read the rules out of a config document's `approval_rules` list.
    ///
    /// Tolerant the way the loaders are: a malformed document, a
    /// missing key, or one unreadable entry leaves the rules it could
    /// read rather than failing the run. A hand-edited list with a bad
    /// line is a file to fix, not a reason to stop the conversation.
    pub fn from_config(document: &serde_json::Value) -> Rules {
        let Some(entries) = document
            .get("approval_rules")
            .and_then(|list| list.as_array())
        else {
            return Rules::default();
        };
        Rules {
            rules: entries.iter().filter_map(Rule::from_value).collect(),
        }
    }

    /// Whether anything is remembered. A surface with nothing to show
    /// says so rather than rendering an empty list.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rules in the file's order, which is the order they were
    /// remembered.
    pub fn list(&self) -> &[Rule] {
        &self.rules
    }

    /// Whether a remembered rule already covers this call.
    pub fn covers(&self, call: &ToolCall) -> bool {
        self.rules.iter().any(|rule| rule.covers_call(call))
    }
}

/// Whether "don't ask again" is a promise this class of call can keep.
///
/// Two classes are out, and they are the same two
/// [`Gate::approves_without_asking`] already refuses to wave through:
/// the always-ask class and the destructive class. Hard rule 7 wants
/// approval in the current turn, and a remembered rule is by
/// construction NOT the current turn — so a rule covering either class
/// would be a way to retire the owner's house rule by clicking a
/// button once. The seat reads this to decide whether to offer the
/// choice at all, and the writer reads it again before it stores one.
pub fn allowlistable(call: &ToolCall) -> bool {
    !asks_regardless(call) && !Gate::is_destructive(call)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Approver;
    use crate::api::FunctionCall;

    /// The always-ask class sits above the modes: a call in it asks
    /// even under `yes`, because its blast radius is the machine
    /// itself. This is the ruling, pinned.
    #[test]
    fn a_computer_use_call_asks_even_under_yes() {
        let yes = Gate::new(Mode::Yes);
        let desktop = call("computer_use", r#"{"action":"click","x":10,"y":20}"#);
        assert!(
            !yes.approves_without_asking(&desktop),
            "computer use asks even under yes — no execution mode waives it"
        );
        assert!(asks_regardless(&desktop));
        // The same call forwarded by a subagent, whose name carries the
        // arrow: the prefix is what the prompt reads, not what the rule
        // keys on.
        let forwarded = call(
            &format!("{}computer_use", crate::tools::subagent_call_prefix()),
            r#"{"action":"screenshot"}"#,
        );
        assert!(asks_regardless(&forwarded));
        assert!(!yes.approves_without_asking(&forwarded));
        // The ordinary tools are untouched by the class.
        let grep = call("bash", r#"{"command":"grep -n pins src/route.rs"}"#);
        assert!(!asks_regardless(&grep));
        assert!(yes.approves_without_asking(&grep));
    }

    /// A server's destructive hint forces the prompt path even under
    /// `yes`, and never touches another call.
    #[test]
    fn a_destructive_hint_forces_the_prompt_path_and_never_grants() {
        let mut yes = Gate::new(Mode::Yes);
        let danger = call("mcp__x__danger", "{}");
        assert!(yes.approves_without_asking(&danger));
        yes.annotate(&danger, true);
        assert!(!yes.approves_without_asking(&danger));
        assert!(!yes.approve(&danger));
        assert!(yes
            .denial_reason()
            .unwrap_or_default()
            .contains("destructive"));
        // The hint is per call: another call is unaffected, and a
        // read-only hint grants nothing.
        let other = ToolCall {
            id: "c2".to_string(),
            ..danger.clone()
        };
        assert!(yes.approves_without_asking(&other));
        let mut read_only = Gate::new(Mode::Yes);
        let quiet = call("mcp__x__read", "{}");
        read_only.annotate(&quiet, false);
        assert!(read_only.approves_without_asking(&quiet));
    }

    /// A tool call as the API issues one: the arguments stay the JSON
    /// text the model produced, exactly as the gate receives them.
    fn call(name: &str, arguments: &str) -> ToolCall {
        ToolCall {
            id: "c1".to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// A `bash` call running `command`, encoded through serde so the
    /// quoting in the test is never the thing under test.
    fn bash(command: &str) -> ToolCall {
        let arguments = format!(
            "{{\"command\":{}}}",
            serde_json::to_string(command).unwrap()
        );
        call("bash", &arguments)
    }

    /// The call as a child's approver forwards it: renamed with the
    /// label — once per delegation level — and its real arguments
    /// wrapped in the goal/tool envelope `subagents::labelled` builds.
    fn forwarded(name: &str, arguments: &str, depth: usize) -> ToolCall {
        let prefix = crate::tools::subagent_call_prefix().repeat(depth);
        let wrapped = serde_json::json!({
            "goal": "the child's goal",
            "tool": name,
            "arguments": arguments,
        })
        .to_string();
        call(&format!("{prefix}{name}"), &wrapped)
    }

    #[test]
    fn ask_approves_nothing_without_asking() {
        let gate = Gate::new(Mode::Ask);
        assert!(!gate.approves_without_asking(&bash("grep -rn fleet src")));
        assert!(!gate.approves_without_asking(&bash("rm -rf /tmp/x")));
        assert!(!gate.approves_without_asking(&call("write_file", r#"{"path":"/tmp/x"}"#)));
    }

    #[test]
    fn a_forwarded_destructive_bash_call_is_destructive_through_its_label() {
        // The class keys on what the call does, not on the name it
        // arrived under (KI-209).
        assert!(Gate::is_destructive(&forwarded(
            "bash",
            r#"{"command":"rm -rf /tmp/x"}"#,
            1
        )));
        assert!(!Gate::is_destructive(&forwarded(
            "bash",
            r#"{"command":"echo hello"}"#,
            1
        )));
    }

    #[test]
    fn yes_guarded_stops_a_forwarded_destructive_call_and_runs_a_benign_one() {
        let gate = Gate::new(Mode::YesGuarded);
        assert!(
            !gate.approves_without_asking(&forwarded(
                "bash",
                r#"{"command":"git push --force"}"#,
                1
            )),
            "the destructive class survives the label"
        );
        assert!(gate.approves_without_asking(&forwarded("bash", r#"{"command":"echo hello"}"#, 1)));
    }

    #[test]
    fn the_always_ask_class_survives_every_depth_of_forwarding() {
        // Depth 1 and depth 2 both land in the class, and no mode waives
        // it (KI-210).
        for depth in [1, 2] {
            let desktop = forwarded("computer_use", r#"{"action":"screenshot"}"#, depth);
            assert!(asks_regardless(&desktop), "depth {depth}");
            assert!(
                !Gate::new(Mode::Yes).approves_without_asking(&desktop),
                "depth {depth}"
            );
            assert!(
                !Gate::new(Mode::YesGuarded).approves_without_asking(&desktop),
                "depth {depth}"
            );
        }
    }

    #[test]
    fn a_forwarded_benign_call_reads_its_real_arguments_not_the_wrapper() {
        // The envelope's own keys never read as the command: the gate
        // sees the inner arguments, and an unparsable one fails closed
        // into the destructive class.
        let gate = Gate::new(Mode::YesGuarded);
        assert!(
            !gate.approves_without_asking(&forwarded("bash", r#"{"command":"rm -rf /tmp/x"}"#, 2)),
            "depth 2 is read through both labels"
        );
    }

    #[test]
    fn yes_approves_everything_the_destructive_class_included() {
        let gate = Gate::new(Mode::Yes);
        assert!(gate.approves_without_asking(&bash("grep -rn fleet src")));
        // The unguarded latch is the point of `yes`, and it is what
        // `yes-guarded` exists to avoid needing on the unattended path.
        assert!(gate.approves_without_asking(&bash("rm -rf /tmp/x")));
        // The class check is mode-independent: the same call the
        // guarded mode stops is still named as destructive here.
        assert!(Gate::is_destructive(&bash("rm -rf /tmp/x")));
    }

    #[test]
    fn yes_guarded_approves_the_ordinary_and_still_asks_about_the_class() {
        let gate = Gate::new(Mode::YesGuarded);
        assert!(gate.approves_without_asking(&bash("grep -rn fleet src")));
        assert!(gate.approves_without_asking(&bash("cargo test --lib approvals")));
        assert!(!gate.approves_without_asking(&bash("rm -rf /tmp/x")));
        assert!(!gate.approves_without_asking(&bash("git push --force origin main")));
        assert!(!gate.approves_without_asking(&bash("cat /srv/app/.env")));
    }

    #[test]
    fn the_gate_reports_the_mode_it_was_built_with() {
        assert_eq!(Gate::new(Mode::Ask).mode(), Mode::Ask);
        assert_eq!(Gate::new(Mode::Yes).mode(), Mode::Yes);
        assert_eq!(Gate::new(Mode::YesGuarded).mode(), Mode::YesGuarded);
    }

    #[test]
    fn an_explicit_yes_flag_decides_regardless_of_the_config() {
        // The flag the owner typed this turn is the decision the run
        // gets, in both directions: it does not narrow a wider config
        // and it does not widen a narrower one into something else.
        assert_eq!(resolve(Some(true), Some("ask")), Mode::Yes);
        assert_eq!(resolve(Some(true), Some("yes-guarded")), Mode::Yes);
        assert_eq!(resolve(Some(true), Some("yes")), Mode::Yes);
        assert_eq!(resolve(Some(true), None), Mode::Yes);
    }

    #[test]
    fn a_config_of_yes_with_no_flag_is_yes() {
        assert_eq!(resolve(Some(false), Some("yes")), Mode::Yes);
        assert_eq!(resolve(None, Some("yes")), Mode::Yes);
    }

    #[test]
    fn an_absent_flag_lets_a_config_of_yes_guarded_decide() {
        assert_eq!(resolve(Some(false), Some("yes-guarded")), Mode::YesGuarded);
        assert_eq!(resolve(None, Some("yes-guarded")), Mode::YesGuarded);
    }

    #[test]
    fn an_unknown_config_word_is_ask_the_fail_closed_reading() {
        assert_eq!(resolve(Some(false), Some("yolo")), Mode::Ask);
        assert_eq!(resolve(Some(false), Some("")), Mode::Ask);
        // A different case is a different word: the match is exact.
        assert_eq!(resolve(Some(false), Some("Yes")), Mode::Ask);
        assert_eq!(resolve(Some(false), None), Mode::Ask);
        assert_eq!(resolve(None, None), Mode::Ask);
    }

    #[test]
    fn a_config_word_survives_surrounding_whitespace() {
        // A trailing newline is a formatting accident in the file, not
        // a word the owner typed.
        assert_eq!(
            resolve(Some(false), Some(" yes-guarded\n")),
            Mode::YesGuarded
        );
        assert_eq!(resolve(Some(false), Some("  yes  ")), Mode::Yes);
    }

    #[test]
    fn the_default_mode_asks() {
        assert_eq!(Mode::default(), Mode::Ask);
        assert_eq!(resolve(Some(false), None), Mode::default());
    }

    #[test]
    fn the_labels_are_exactly_the_three_config_words() {
        assert_eq!(Mode::Ask.label(), "ask");
        assert_eq!(Mode::Yes.label(), "yes");
        assert_eq!(Mode::YesGuarded.label(), "yes-guarded");
    }

    #[test]
    fn malformed_bash_arguments_are_destructive_the_fail_closed_reading() {
        // Not JSON at all.
        assert!(Gate::is_destructive(&call("bash", "not json")));
        // Valid JSON that names no command.
        assert!(Gate::is_destructive(&call("bash", r#"{"timeout":5}"#)));
        // A command that is not a string.
        assert!(Gate::is_destructive(&call("bash", r#"{"command":42}"#)));
        // And the guarded mode refuses all three, rather than reading an
        // unreadable command as an ordinary one.
        let gate = Gate::new(Mode::YesGuarded);
        assert!(!gate.approves_without_asking(&call("bash", "not json")));
        assert!(!gate.approves_without_asking(&call("bash", r#"{"timeout":5}"#)));
    }

    #[test]
    fn a_non_bash_tool_is_never_destructive() {
        // `write_file` can overwrite a path, but the path is the run's
        // own business: the class is about the shell's reach, and only
        // bash has it.
        assert!(!Gate::is_destructive(&call(
            "write_file",
            r#"{"path":"/etc/passwd","content":"x"}"#
        )));
        // Even a tool whose own arguments name a credential file.
        assert!(!Gate::is_destructive(&call(
            "read_file",
            r#"{"path":"~/.ssh/id_rsa"}"#
        )));
        assert!(!Gate::is_destructive(&call(
            "list_dir",
            r#"{"path":"/srv/.env"}"#
        )));
        // And unreadable arguments on a non-bash tool are not
        // destructive either: there is no command to misread, which is
        // the distinction the two branches of `is_destructive` turn on.
        assert!(!Gate::is_destructive(&call("write_file", "not json")));
    }

    #[test]
    fn every_marker_in_the_destructive_list_fires_on_a_representative_command() {
        // One representative command per marker. The table is the
        // constant's own proof: a marker that matches nothing is a gate
        // that lies about what it covers, and this test is what catches
        // it the day the list gains an entry nobody exercised.
        const CASES: &[(&str, &str)] = &[
            ("rm -rf", "rm -rf /tmp/build"),
            ("rm -r ", "rm -r /tmp/build"),
            ("git push --force", "git push --force origin main"),
            ("git push -f", "git push -f origin main"),
            ("git reset --hard", "git reset --hard HEAD~3"),
            ("git clean -", "git clean -fd"),
            (
                "git filter-branch",
                "git filter-branch --tree-filter 'rm -f secret' -- --all",
            ),
            ("dd if=", "dd if=/dev/zero of=/dev/sda bs=4M"),
            ("mkfs", "mkfs.ext4 /dev/sda1"),
            ("> /dev/sd", "echo wipe > /dev/sda"),
            ("chmod -R 000", "chmod -R 000 /var/lib/data"),
            ("/.env", "cat /srv/app/.env"),
            ("id_rsa", "cat ~/.ssh/id_rsa"),
            (".pem", "cat /etc/ssl/private/server.pem"),
            ("credentials", "cat ~/.aws/credentials"),
        ];
        assert_eq!(
            CASES.len(),
            DESTRUCTIVE.len(),
            "every marker has a case here"
        );
        for &(marker, command) in CASES {
            assert!(
                DESTRUCTIVE.contains(&marker),
                "`{marker}` is not in DESTRUCTIVE"
            );
            assert!(
                command.contains(marker),
                "the case for `{marker}` does not contain it"
            );
            assert!(
                Gate::is_destructive(&bash(command)),
                "`{command}` should be in the class via `{marker}`"
            );
        }
    }

    #[test]
    fn an_interactive_rebase_is_not_in_the_destructive_class() {
        // Named in the list's docs as deliberately excluded: a rebase
        // is a review surface, and stopping it would teach the owner to
        // answer the prompt without reading it.
        assert!(!Gate::is_destructive(&bash("git rebase -i HEAD~5")));
        assert!(!Gate::is_destructive(&bash("git push origin main")));
        assert!(!Gate::is_destructive(&bash("git status --short")));
        assert!(!Gate::is_destructive(&bash("cargo test --lib approvals")));
    }

    #[test]
    fn a_guarded_denial_names_the_mode_the_class_and_the_command() {
        let gate = Gate::new(Mode::YesGuarded);
        let text = gate.denial_text(&bash("rm -rf /tmp/x"));
        assert!(text.contains("yes-guarded"), "{text}");
        assert!(text.contains("destructive"), "{text}");
        assert!(text.contains("rm -rf /tmp/x"), "{text}");
        // The sentence an AFK owner reads in the log has to say why the
        // run stopped without them, not merely that it did.
        assert!(text.contains("house rule"), "{text}");
    }

    #[test]
    fn a_denial_for_an_unreadable_call_says_so_rather_than_naming_nothing() {
        let gate = Gate::new(Mode::YesGuarded);
        let text = gate.denial_text(&call("bash", "not json"));
        assert!(text.contains("could not be read"), "{text}");
        assert!(text.contains("yes-guarded"), "{text}");
    }
}
