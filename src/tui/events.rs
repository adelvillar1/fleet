//! Key presses to actions. The mapping is a pure function of the pressed
//! key and the app's mode, so it is tested without a terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::agent::Approval;
use super::app::{Action, App, Mode};

/// Translate one key press for the app's current mode.
pub fn action_for(app: &App, key: &KeyEvent) -> Option<Action> {
    // Only press events: some terminals also deliver release and repeat.
    if key.kind != ratatui::crossterm::event::KeyEventKind::Press {
        return None;
    }
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    // The one key every mode honors.
    if control && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    match app.mode {
        Mode::Chat => chat_action(app, key, control, alt),
        Mode::Sessions => match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Action::Move(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::Move(1)),
            KeyCode::Enter => Some(Action::Pick),
            KeyCode::Esc => Some(Action::CloseOverlay),
            KeyCode::Char('q') if !control => Some(Action::Quit),
            _ => None,
        },
        // The modal answers or denies on any key, like the console prompt.
        Mode::Approval => Some(match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::Approve(Approval::Once),
            KeyCode::Char('a') | KeyCode::Char('A') => Action::Approve(Approval::Always),
            _ => Action::Approve(Approval::Denied),
        }),
    }
}

/// The chat pane's keys: typing, editing, submitting, overlays, scrolling.
fn chat_action(app: &App, key: &KeyEvent, control: bool, alt: bool) -> Option<Action> {
    match key.code {
        KeyCode::Enter => {
            // Alt+Enter types a newline; plain Enter submits.
            if alt {
                Some(Action::Input('\n'))
            } else {
                Some(Action::Submit)
            }
        }
        // Editing keys: the cursor moves, deletes land at it.
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Home => Some(Action::CursorHome),
        KeyCode::End => Some(Action::CursorEnd),
        KeyCode::Delete => Some(Action::Delete),
        KeyCode::Backspace if alt => Some(Action::DeleteWord),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char('w') if control => Some(Action::DeleteWord),
        KeyCode::Tab => Some(Action::Input('\t')),
        KeyCode::Up => Some(Action::Scroll(1)),
        KeyCode::Down => Some(Action::Scroll(-1)),
        KeyCode::PageUp => Some(Action::Scroll(10)),
        KeyCode::PageDown => Some(Action::Scroll(-10)),
        KeyCode::Char('o') if control => Some(Action::OpenSessions),
        KeyCode::Char(character) => {
            if control {
                // Ctrl+key combinations are commands or nothing, never text.
                None
            } else if character == 'q' && app.input.is_empty() {
                // `q` quits from an empty prompt and types otherwise.
                Some(Action::Quit)
            } else {
                Some(Action::Input(character))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventKind;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn control(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn app() -> App {
        App::new("m", "s")
    }

    #[test]
    fn typing_goes_to_the_input_and_q_quits_only_when_empty() {
        let mut with_text = app();
        assert_eq!(action_for(&with_text, &key(KeyCode::Char('a'))), Some(Action::Input('a')));
        with_text.apply(Action::Input('a'));
        // With text in the box, `q` is just a letter.
        assert_eq!(action_for(&with_text, &key(KeyCode::Char('q'))), Some(Action::Input('q')));
        // From an empty prompt, `q` quits.
        let empty = app();
        assert_eq!(action_for(&empty, &key(KeyCode::Char('q'))), Some(Action::Quit));
    }

    #[test]
    fn enter_submits_and_alt_enter_types_a_newline() {
        let app = app();
        assert_eq!(action_for(&app, &key(KeyCode::Enter)), Some(Action::Submit));
        let alt_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
        assert_eq!(action_for(&app, &alt_enter), Some(Action::Input('\n')));
    }

    #[test]
    fn ctrl_c_always_quits_and_ctrl_o_opens_the_picker() {
        let app = app();
        assert_eq!(action_for(&app, &control(KeyCode::Char('c'))), Some(Action::Quit));
        assert_eq!(action_for(&app, &control(KeyCode::Char('o'))), Some(Action::OpenSessions));
        // Other control combinations are not text.
        assert_eq!(action_for(&app, &control(KeyCode::Char('x'))), None);
    }

    #[test]
    fn scrolling_keys_move_the_conversation() {
        let app = app();
        assert_eq!(action_for(&app, &key(KeyCode::Up)), Some(Action::Scroll(1)));
        assert_eq!(action_for(&app, &key(KeyCode::PageUp)), Some(Action::Scroll(10)));
        assert_eq!(action_for(&app, &key(KeyCode::PageDown)), Some(Action::Scroll(-10)));
    }

    #[test]
    fn the_editing_keys_move_and_delete_at_the_cursor() {
        let typing = app();
        assert_eq!(action_for(&typing, &key(KeyCode::Left)), Some(Action::CursorLeft));
        assert_eq!(action_for(&typing, &key(KeyCode::Right)), Some(Action::CursorRight));
        assert_eq!(action_for(&typing, &key(KeyCode::Home)), Some(Action::CursorHome));
        assert_eq!(action_for(&typing, &key(KeyCode::End)), Some(Action::CursorEnd));
        assert_eq!(action_for(&typing, &key(KeyCode::Delete)), Some(Action::Delete));
        assert_eq!(action_for(&typing, &key(KeyCode::Backspace)), Some(Action::Backspace));
        // Word delete answers to alt+backspace and to ctrl+w.
        let alt_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT);
        assert_eq!(action_for(&typing, &alt_backspace), Some(Action::DeleteWord));
        assert_eq!(
            action_for(&typing, &control(KeyCode::Char('w'))),
            Some(Action::DeleteWord)
        );
        // The picker ignores the chat's editing keys.
        let mut picking = app();
        picking.open_sessions(Vec::new());
        assert_eq!(action_for(&picking, &key(KeyCode::Left)), None);
        assert_eq!(action_for(&picking, &key(KeyCode::Delete)), None);
    }

    #[test]
    fn picker_keys_navigate_and_escape_closes() {
        let mut app = app();
        app.open_sessions(Vec::new());
        assert_eq!(action_for(&app, &key(KeyCode::Down)), Some(Action::Move(1)));
        assert_eq!(action_for(&app, &key(KeyCode::Char('j'))), Some(Action::Move(1)));
        assert_eq!(action_for(&app, &key(KeyCode::Enter)), Some(Action::Pick));
        assert_eq!(action_for(&app, &key(KeyCode::Esc)), Some(Action::CloseOverlay));
        assert_eq!(action_for(&app, &key(KeyCode::Char('q'))), Some(Action::Quit));
    }

    #[test]
    fn the_approval_modal_answers_y_and_a_and_denies_the_rest() {
        let mut app = app();
        app.show_approval("bash", "ls");
        assert_eq!(
            action_for(&app, &key(KeyCode::Char('y'))),
            Some(Action::Approve(Approval::Once))
        );
        assert_eq!(
            action_for(&app, &key(KeyCode::Char('a'))),
            Some(Action::Approve(Approval::Always))
        );
        assert_eq!(
            action_for(&app, &key(KeyCode::Char('n'))),
            Some(Action::Approve(Approval::Denied))
        );
        assert_eq!(
            action_for(&app, &key(KeyCode::Enter)),
            Some(Action::Approve(Approval::Denied))
        );
    }

    #[test]
    fn release_events_are_ignored() {
        let app = app();
        let release = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE).state,
        };
        assert_eq!(action_for(&app, &release), None);
    }
}
