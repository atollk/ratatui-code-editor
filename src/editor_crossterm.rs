use crate::actions::*;
use crate::editor::Editor;
use crate::selection::SelectionSnap;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui_core::layout::Rect;

fn key_to_action(key: &KeyEvent) -> Option<DefaultAction> {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let _alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Char('÷') => Some(DefaultAction::ToggleComment),
        KeyCode::Char('z') if ctrl => Some(DefaultAction::Undo),
        KeyCode::Char('y') if ctrl => Some(DefaultAction::Redo),
        KeyCode::Char('c') if ctrl => Some(DefaultAction::Copy),
        KeyCode::Char('v') if ctrl => Some(DefaultAction::Paste),
        KeyCode::Char('x') if ctrl => Some(DefaultAction::Cut),
        KeyCode::Char('k') if ctrl => Some(DefaultAction::DeleteLine),
        KeyCode::Char('d') if ctrl => Some(DefaultAction::Duplicate),
        KeyCode::Char('a') if ctrl => Some(DefaultAction::SelectAll),
        KeyCode::Left => Some(DefaultAction::MoveLeft { shift }),
        KeyCode::Right => Some(DefaultAction::MoveRight { shift }),
        KeyCode::Up => Some(DefaultAction::MoveUp { shift }),
        KeyCode::Down => Some(DefaultAction::MoveDown { shift }),
        KeyCode::Backspace => Some(DefaultAction::Delete),
        KeyCode::Enter => Some(DefaultAction::InsertNewline),
        KeyCode::Char(c) => Some(DefaultAction::InsertText {
            text: c.to_string()
        }),
        KeyCode::Tab => Some(DefaultAction::Indent),
        KeyCode::BackTab => Some(DefaultAction::UnIndent),
        _ => None,
    }
}

impl<'a> Editor<'a> {
    pub fn input(&mut self, key: &KeyEvent, area: &Rect) -> Result<()> {
        if let Some(action) = key_to_action(key) {
            self.apply(action);
        }
        self.focus(&area);
        Ok(())
    }

    pub fn mouse(&mut self, mouse: MouseEvent, area: &Rect) -> Result<()> {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(),
            MouseEventKind::ScrollDown => self.scroll_down(area.height as usize),
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);
                if let Some(cursor) = pos {
                    self.handle_mouse_down(cursor);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Auto-scroll when dragging on the last or first visible row
                if mouse.row == area.top() {
                    self.scroll_up();
                }
                if mouse.row == area.bottom().saturating_sub(1) {
                    self.scroll_down(area.height as usize);
                }
                let pos = self.cursor_from_mouse(mouse.column, mouse.row, area);
                if let Some(cursor) = pos {
                    self.handle_mouse_drag(cursor);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.selection_snap = SelectionSnap::None;
            }
            _ => {}
        }
        Ok(())
    }
}
