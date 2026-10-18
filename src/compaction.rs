//! Context-window management: summarize-and-continue under a token
//! budget.
//!
//! A long conversation outgrows the model's window. The fork's
//! `packages/compaction` answers that with a replay-aware compaction
//! backend: a region of the surface is summarized by the same routed
//! model, the summary replaces the region, a recent tail stays
//! verbatim, and the whole transaction is recorded in the session log
//! so replay reproduces every request that followed. This module is
//! that port — the vocabulary and the arithmetic, with no dependency on
//! the fork (hard rule 6).
//!
//! Three things live here and nowhere else: the budget and the tail
//! size that decide *when* and *how much*, the estimate that stands in
//! when the API does not report its own count, and the frame the summary
//! rides in. The agent owns the request; the session log owns the
//! record.

use crate::api::{Message, Role};

/// The floor under every derived budget, and the budget itself for a
/// model whose window is not declared. DeepSeek's window is 128k, so
/// this fires at roughly 80% of it — early enough that the request that
/// crosses it still fits, late enough that a run which never approaches
/// the window pays nothing.
pub const COMPACTION_TOKEN_BUDGET: u64 = 100_000;

/// The fraction of a context window the conversation is folded at. Three
/// fifths leaves two fifths for the fold's own summary and the turns
/// that follow it, which is the headroom the window is actually spent on.
const WINDOW_FRACTION_NUM: u64 = 3;
const WINDOW_FRACTION_DEN: u64 = 5;

/// The headroom kept below a window's edge. The fold's summary request
/// carries the region being summarized plus a long response, so the
/// budget never reaches the edge itself — but it is a reserve for a
/// *typical* long response, not for the model's maximum selectable
/// output: a model that can emit 256k tokens does not emit them every
/// turn, and a budget sized to the maximum would fold every turn
/// instead.
const WINDOW_RESERVE: u64 = 32_000;

/// The token budget one run compacts at, given the serving model's
/// declared context window.
///
/// A fixed budget is the behaviour every run had before windows were
/// declared, and it is tuned for the smallest window in the roster: on
/// step-5-preview's 1M window it folds ten times more often than the
/// window requires, and every fold costs a summary request and a
/// lossier conversation. So the budget follows the window when it is
/// known — a fraction of it, held at or above the floor for small
/// windows and held below the edge for every one — and falls back to
/// the floor when it is not, which is the behaviour a run with an
/// undeclared model has always had.
pub fn budget_for(window: Option<u64>) -> u64 {
    let Some(window) = window.filter(|window| *window > 0) else {
        return COMPACTION_TOKEN_BUDGET;
    };
    let fraction = window * WINDOW_FRACTION_NUM / WINDOW_FRACTION_DEN;
    let edge = window.saturating_sub(WINDOW_RESERVE);
    fraction.max(COMPACTION_TOKEN_BUDGET).min(edge)
}

/// How many of the most recent turns stay verbatim after a compaction.
/// The summary is lossy by construction; the tail is not, so the work in
/// flight — the call being made and its answer — survives the fold
/// exactly as it happened.
pub const COMPACTION_TAIL_TURNS: usize = 4;

/// The instruction the compaction request carries as its final message.
///
/// The fork delivered the same directive after the replayed prefix
/// rather than as a separate summarizer system prompt, which keeps the
/// auxiliary call a genuine prefix of the last routed request so the
/// provider's cache is reused. Fleet carries the conversation being
/// compacted and this instruction and nothing else: the request has no
/// tools, because a compaction request has nothing to execute.
///
/// It is a compiled-in constant like the system prompt and the tool
/// declarations — reconstructed per compaction request, never logged
/// (hard rule 5). What makes the request reconstructable from the log is
/// the `compaction` record beside the messages it replaces.
pub const COMPACTION_INSTRUCTION: &str = "You are now acting as a compaction engine for this coding assistant. Condense the conversation ABOVE into a structured summary that lets another model resume the work with no loss of essential context.\n\nUse terse bullets, not prose paragraphs. Preserve exact file paths, commands, error strings, identifiers, numeric values, and function signatures. Capture the user's requests and any corrections faithfully. If the conversation already contains an earlier summary, it is a prior checkpoint: preserve what is still true, drop what is stale, and merge newer information into a single consolidated summary.\n\nDo not mention this summarization request or that the context was compacted. Output only the summary: do not call any tool and take no other action.";

/// Characters per token in the fallback estimate. The usual rule of
/// thumb for English prose and code mixed; it is an estimate, and the
/// only one available when a provider does not report its own count.
const CHARS_PER_TOKEN: u64 = 4;

/// The chars/4 estimate of `messages`: the total characters every
/// message carries — its text, its tool-call names, ids, and arguments —
/// divided by [`CHARS_PER_TOKEN`].
///
/// This is the fallback, never the preference: the API's own
/// `prompt_tokens` is exact and is used whenever the last response
/// carried one. The estimate counts the messages and not the system
/// prompt or the tool declarations, so it reads slightly under the
/// API's count; the budget has room for that.
/// The fallback estimate with the system prompt counted beside the
/// messages. KI-081: the API's own `prompt_tokens` counts the system
/// prompt and the tool declarations — and the declarations ride inside
/// the assembled prompt — so an estimate of the messages alone reads
/// under the real count by exactly their size, which made the budget
/// check optimistic. The prompt's own chars/4 is the same estimate's
/// rule applied to the one string that carries both.
pub fn estimate_with_prompt(messages: &[Message], system_prompt: &str) -> u64 {
    estimate_tokens(messages) + (system_prompt.chars().count() as u64) / 4
}

pub fn estimate_tokens(messages: &[Message]) -> u64 {
    let chars: usize = messages
        .iter()
        .map(|message| {
            let text = message
                .content
                .as_deref()
                .map_or(0, |content| content.chars().count());
            let calls: usize = message
                .tool_calls
                .iter()
                .map(|call| {
                    call.id.chars().count()
                        + call.kind.chars().count()
                        + call.function.name.chars().count()
                        + call.function.arguments.chars().count()
                })
                .sum();
            let answered = message
                .tool_call_id
                .as_deref()
                .map_or(0, |id| id.chars().count());
            text + calls + answered
        })
        .sum();
    chars as u64 / CHARS_PER_TOKEN
}

/// The message the summary rides in: a user-role message whose text is a
/// delimited frame naming the turns it stands in for.
///
/// The frame is what makes the compacted shape readable to a human
/// reading the log and unambiguous to a model reading the request: the
/// summary is established background, not something the user just said.
pub fn summary_frame(first_turn: u64, last_turn: u64, summary: &str) -> String {
    format!(
        "Summary of turns {first_turn} through {last_turn}, compacted to fit the window:\n\n{summary}"
    )
}

/// The index the verbatim tail starts at: the most recent
/// [`COMPACTION_TAIL_TURNS`] assistant messages and everything from the
/// first of them onward, or 0 when the conversation holds no more turns
/// than that.
///
/// The tail starts on an assistant message, never on a tool result: a
/// message list that opens with a `tool` role is not a valid sequence
/// for the API, which is the KI-016 failure in a new shape. Everything
/// before this index is what a compaction replaces.
///
/// The agent and the session log's replay both call this one function
/// on the same message list, so the split a compaction records and the
/// split a replay reconstructs cannot drift.
pub fn tail_start(messages: &[Message]) -> usize {
    let assistants = messages
        .iter()
        .filter(|message| message.role == Role::Assistant)
        .count();
    if assistants <= COMPACTION_TAIL_TURNS {
        return 0;
    }
    // 1-based ordinal of the first tail turn's assistant message.
    let wanted = assistants - COMPACTION_TAIL_TURNS + 1;
    let mut seen = 0;
    for (index, message) in messages.iter().enumerate() {
        if message.role == Role::Assistant {
            seen += 1;
            if seen == wanted {
                return index;
            }
        }
    }
    0
}

/// How many turns a compaction record's range names. A range is
/// inclusive, and a record whose first turn is after its last — which
/// cannot be written by the agent — names one turn rather than
/// underflowing.
pub fn turns_in_range(first_turn: u64, last_turn: u64) -> usize {
    last_turn.saturating_sub(first_turn).saturating_add(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{FunctionCall, ToolCall};

    fn text(role: Role, content: &str) -> Message {
        Message::text(role, content)
    }

    fn assistant() -> Message {
        Message {
            role: Role::Assistant,
            content: Some("working".to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            images: Vec::new(),
        }
    }

    fn tool_result() -> Message {
        Message::tool_result("call-1", "the output")
    }

    fn calling() -> Message {
        Message {
            role: Role::Assistant,
            content: None,
            tool_calls: vec![ToolCall {
                id: "call-1".to_string(),
                kind: "function".to_string(),
                function: FunctionCall {
                    name: "bash".to_string(),
                    arguments: r#"{"command":"ls -la"}"#.to_string(),
                },
            }],
            tool_call_id: None,
            images: Vec::new(),
        }
    }

    /// One full turn: an assistant answer and the tool result that
    /// answers its call.
    fn turn() -> Vec<Message> {
        vec![calling(), tool_result()]
    }

    #[test]
    fn the_tail_boundary_counts_assistant_turns_not_messages() {
        // A turn is one model request, so a turn that carried no tool call
        // still counts as one: five assistant answers and no tool results
        // at all put the boundary before the second answer.
        let mut messages = vec![text(Role::User, "the task")];
        for _ in 0..5 {
            messages.push(assistant());
        }
        let start = tail_start(&messages);
        assert_eq!(start, 2, "the task plus the first answer are replaced");
        assert_eq!(messages[start].role, Role::Assistant);
        assert_eq!(
            messages[start..]
                .iter()
                .filter(|m| m.role == Role::Assistant)
                .count(),
            COMPACTION_TAIL_TURNS
        );
    }

    #[test]
    fn the_budget_and_the_tail_are_named_constants() {
        assert_eq!(COMPACTION_TOKEN_BUDGET, 100_000);
        assert_eq!(COMPACTION_TAIL_TURNS, 4);
    }

    #[test]
    fn the_budget_follows_the_window_and_falls_back_to_the_floor() {
        // No window declared: the floor, which is the behaviour every run
        // had before windows were declared.
        assert_eq!(budget_for(None), 100_000);
        assert_eq!(budget_for(Some(0)), 100_000);
        // A small window is held at the floor and never past the edge:
        // 60% of 128k is 76,800, so the floor wins, and the edge is what
        // keeps it under the window itself.
        assert_eq!(budget_for(Some(128_000)), 96_000);
        assert_eq!(budget_for(Some(65_536)), 33_536);
        // The roster's own stepfun window: three fifths of it, and the
        // edge is nowhere near.
        assert_eq!(budget_for(Some(262_144)), 157_286);
        // step-5-preview's 1M window: the case this was built for. The
        // fixed budget folded this model at 100k — ten times more often
        // than its window requires.
        assert_eq!(budget_for(Some(1_024_000)), 614_400);
        // A window whose three fifths pass the edge is held at the edge.
        assert_eq!(budget_for(Some(60_000)), 28_000);
        // Every derived budget stays inside its window, which is what
        // makes the request that crosses it still fit.
        for window in [
            65_536u64, 128_000, 262_144, 400_000, 1_000_000, 1_024_000, 1_048_576,
        ] {
            let budget = budget_for(Some(window));
            assert!(budget <= window, "{budget} past {window}");
            assert!(budget > 0, "{budget}");
        }
    }

    #[test]
    fn the_estimate_counts_every_character_a_message_carries() {
        // Empty: nothing.
        assert_eq!(estimate_tokens(&[]), 0);
        // 400 characters of prose reads as 100 tokens.
        assert_eq!(estimate_tokens(&[text(Role::User, &"x".repeat(400))]), 100);
        // A tool call's name, id, and arguments count too — they are
        // tokens on the wire, and a conversation of pure tool traffic
        // must not estimate as empty.
        let calls = estimate_tokens(&[calling()]);
        assert!(calls > 0, "{calls}");
        // The tool result's text and its call id both count.
        let answered = estimate_tokens(&[tool_result()]);
        assert_eq!(
            answered,
            ("the output".chars().count() + "call-1".chars().count()) as u64 / 4
        );
    }

    /// KI-081: the estimate counts the system prompt and the tool
    /// declarations beside the messages — the prompt they ride in is the
    /// assembled one, so the declarations are in it. The API's own count
    /// includes both, and an estimate that read under it made the budget
    /// check optimistic by exactly their size.
    #[test]
    fn the_estimate_counts_the_prompt_and_the_declarations_it_carries() {
        let messages = vec![text(Role::User, &"x".repeat(400))];
        let bare = estimate_tokens(&messages);
        // 400 characters of prompt reads as 100 tokens beside the
        // message's own 100.
        assert_eq!(
            estimate_with_prompt(&messages, &"y".repeat(400)),
            bare + 100
        );
        // An empty prompt changes nothing.
        assert_eq!(estimate_with_prompt(&messages, ""), bare);
        // The declarations ride inside the assembled prompt, so counting
        // the prompt counts them.
        let declarations = "z".repeat(800);
        assert_eq!(estimate_with_prompt(&messages, &declarations), bare + 200);
    }

    #[test]
    fn the_estimate_divides_by_four() {
        // Not a rounding function: 399 characters is still 99 tokens.
        assert_eq!(estimate_tokens(&[text(Role::User, &"x".repeat(399))]), 99);
    }

    #[test]
    fn the_frame_names_the_turns_it_stands_in_for() {
        let frame = summary_frame(3, 9, "- did the thing\n- then the other thing");
        assert_eq!(
            frame,
            "Summary of turns 3 through 9, compacted to fit the window:\n\n- did the thing\n- then the other thing"
        );
        // The summary's own text rides verbatim: a frame that rewrote it
        // would make the log's summary and the request's summary differ.
        assert!(frame.ends_with("- then the other thing"));
    }

    #[test]
    fn a_short_conversation_has_no_tail_boundary() {
        // Four turns exactly: the whole conversation is the tail, and a
        // compaction would have nothing to replace.
        let mut messages = vec![text(Role::User, "the task")];
        for _ in 0..COMPACTION_TAIL_TURNS {
            messages.extend(turn());
        }
        assert_eq!(tail_start(&messages), 0);
        // Fewer still.
        assert_eq!(tail_start(&[text(Role::User, "the task")]), 0);
        assert_eq!(tail_start(&[]), 0);
    }

    #[test]
    fn the_tail_starts_on_an_assistant_message_never_a_tool_result() {
        // Six turns: the tail is the last four, so the boundary lands on
        // the third turn's assistant message.
        let mut messages = vec![text(Role::User, "the task")];
        for _ in 0..6 {
            messages.extend(turn());
        }
        let start = tail_start(&messages);
        assert_eq!(messages[start].role, Role::Assistant);
        // Exactly four assistant messages survive.
        assert_eq!(
            messages[start..]
                .iter()
                .filter(|m| m.role == Role::Assistant)
                .count(),
            COMPACTION_TAIL_TURNS
        );
        // And the tool results that follow their own assistant ride
        // along with it, so no call is orphaned.
        assert_eq!(messages[start + 1].role, Role::Tool);
        // One more turn moves the boundary by exactly one turn.
        messages.extend(turn());
        let later = tail_start(&messages);
        assert_eq!(messages[later].role, Role::Assistant);
        assert_eq!(
            later,
            start + 2,
            "one more turn is one assistant plus its result"
        );
    }

    #[test]
    fn a_turn_count_is_inclusive_and_cannot_underflow() {
        assert_eq!(turns_in_range(1, 1), 1);
        assert_eq!(turns_in_range(1, 6), 6);
        assert_eq!(turns_in_range(3, 9), 7);
        // A record that names its range backwards still counts one turn.
        assert_eq!(turns_in_range(9, 3), 1);
    }

    #[test]
    fn the_instruction_asks_for_a_summary_and_no_tool_call() {
        assert!(COMPACTION_INSTRUCTION.contains("compaction engine"));
        assert!(COMPACTION_INSTRUCTION.contains("do not call any tool"));
        // The fork's prior-checkpoint rule: a second compaction folds the
        // first summary rather than copying it forward.
        assert!(COMPACTION_INSTRUCTION.contains("prior checkpoint"));
    }
}
