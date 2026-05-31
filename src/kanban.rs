// Allow dead code in src/kanban.rs when the kanban feature is disabled in the build.
// This preserves the pure data structure for snapshot/session compatibility.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KanbanState {
    pub items: Vec<crate::api::schema::KanbanItem>,
    pub selected_col: usize,
    pub selected_row: usize,
    pub detail_uuid: Option<String>,
    pub detail_scroll: u16,
    pub detail_horizontal_scroll: u16,
}

impl KanbanState {
    pub fn new(items: Vec<crate::api::schema::KanbanItem>) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }

    pub fn add_item(
        &mut self,
        title: String,
        description: Option<String>,
        status: Option<crate::api::schema::KanbanStatus>,
        terminal_id: Option<String>,
    ) -> crate::api::schema::KanbanItem {
        let item = crate::api::schema::KanbanItem {
            uuid: uuid::Uuid::new_v4().to_string(),
            title,
            description: description.unwrap_or_default(),
            status: status.unwrap_or(crate::api::schema::KanbanStatus::Todo),
            terminal_id,
        };
        self.items.push(item.clone());
        item
    }

    pub fn update_item(
        &mut self,
        uuid: &str,
        title: Option<String>,
        description: Option<String>,
        status: Option<crate::api::schema::KanbanStatus>,
        terminal_id: Option<String>,
        clear_terminal_id: Option<bool>,
    ) -> Option<crate::api::schema::KanbanItem> {
        let item = self.items.iter_mut().find(|it| it.uuid == uuid)?;
        if let Some(t) = title {
            item.title = t;
        }
        if let Some(d) = description {
            item.description = d;
        }
        if let Some(s) = status {
            item.status = s;
        }
        if clear_terminal_id.unwrap_or(false) {
            item.terminal_id = None;
        } else if terminal_id.is_some() {
            item.terminal_id = terminal_id;
        }
        Some(item.clone())
    }

    pub fn delete_item(&mut self, uuid: &str) -> Option<crate::api::schema::KanbanItem> {
        let pos = self.items.iter().position(|it| it.uuid == uuid)?;
        Some(self.items.remove(pos))
    }

    pub fn clear_dead_terminals<F>(&mut self, terminal_exists: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        let mut tids_to_clear = Vec::new();
        for item in self.items.iter() {
            if let Some(ref tid) = item.terminal_id {
                if !terminal_exists(tid) {
                    tids_to_clear.push(item.uuid.clone());
                }
            }
        }

        let mut any_cleared = false;
        for item in self.items.iter_mut() {
            if tids_to_clear.contains(&item.uuid) {
                item.terminal_id = None;
                any_cleared = true;
            }
        }

        any_cleared
    }

    pub fn items_in_column(&self, col: usize) -> Vec<&crate::api::schema::KanbanItem> {
        let status = match col {
            0 => crate::api::schema::KanbanStatus::Todo,
            1 => crate::api::schema::KanbanStatus::InProgress,
            2 => crate::api::schema::KanbanStatus::NeedReview,
            3 => crate::api::schema::KanbanStatus::Done,
            _ => return vec![],
        };
        self.items
            .iter()
            .filter(|item| item.status == status)
            .collect()
    }

    pub fn move_col_left(&mut self) {
        if self.selected_col > 0 {
            self.selected_col -= 1;
            self.selected_row = 0;
        }
    }

    pub fn move_col_right(&mut self) {
        if self.selected_col < 3 {
            self.selected_col += 1;
            self.selected_row = 0;
        }
    }

    pub fn move_row_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn move_row_down(&mut self) {
        let count = self.items_in_column(self.selected_col).len();
        if count > 0 && self.selected_row < count - 1 {
            self.selected_row += 1;
        }
    }

    pub fn shift_item_left(&mut self) {
        let col = self.selected_col;
        if col == 0 {
            return;
        }
        let items = self.items_in_column(col);
        if let Some(item_to_move) = items.get(self.selected_row) {
            let uuid = item_to_move.uuid.clone();
            let new_status = match col - 1 {
                0 => crate::api::schema::KanbanStatus::Todo,
                1 => crate::api::schema::KanbanStatus::InProgress,
                2 => crate::api::schema::KanbanStatus::NeedReview,
                _ => return,
            };
            self.update_item(&uuid, None, None, Some(new_status), None, None);
            self.selected_col = col - 1;
            let new_count = self.items_in_column(self.selected_col).len();
            self.selected_row = new_count.saturating_sub(1);
        }
    }

    pub fn shift_item_right(&mut self) {
        let col = self.selected_col;
        if col >= 3 {
            return;
        }
        let items = self.items_in_column(col);
        if let Some(item_to_move) = items.get(self.selected_row) {
            let uuid = item_to_move.uuid.clone();
            let new_status = match col + 1 {
                1 => crate::api::schema::KanbanStatus::InProgress,
                2 => crate::api::schema::KanbanStatus::NeedReview,
                3 => crate::api::schema::KanbanStatus::Done,
                _ => return,
            };
            self.update_item(&uuid, None, None, Some(new_status), None, None);
            self.selected_col = col + 1;
            let new_count = self.items_in_column(self.selected_col).len();
            self.selected_row = new_count.saturating_sub(1);
        }
    }

    pub fn delete_selected(&mut self) {
        let col = self.selected_col;
        let items = self.items_in_column(col);
        if let Some(item) = items.get(self.selected_row) {
            let uuid = item.uuid.clone();
            self.delete_item(&uuid);
            let new_count = self.items_in_column(col).len();
            if self.selected_row >= new_count && new_count > 0 {
                self.selected_row = new_count - 1;
            } else if new_count == 0 {
                self.selected_row = 0;
            }
        }
    }

    pub fn set_detail_uuid(&mut self, uuid: Option<String>) {
        self.detail_uuid = uuid;
        self.detail_scroll = 0;
        self.detail_horizontal_scroll = 0;
    }

    pub fn scroll_detail(&mut self, delta: i16, max_scroll: u16) {
        let current = self.detail_scroll as i16;
        self.detail_scroll = current.saturating_add(delta).clamp(0, max_scroll as i16) as u16;
    }

    pub fn scroll_horizontal_detail(&mut self, delta: i16, max_scroll: u16) {
        let current = self.detail_horizontal_scroll as i16;
        self.detail_horizontal_scroll =
            current.saturating_add(delta).clamp(0, max_scroll as i16) as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::KanbanStatus;

    #[test]
    fn test_kanban_state_mutations() {
        let mut state = KanbanState::default();

        // 1. Add items
        let item1 = state.add_item("Task 1".into(), None, Some(KanbanStatus::Todo), None);
        let item2 = state.add_item("Task 2".into(), None, Some(KanbanStatus::InProgress), None);
        assert_eq!(state.items.len(), 2);
        assert_eq!(state.items_in_column(0).len(), 1);
        assert_eq!(state.items_in_column(1).len(), 1);

        // 2. Update item
        let updated = state
            .update_item(
                &item1.uuid,
                Some("New Task 1 Title".into()),
                None,
                Some(KanbanStatus::NeedReview),
                None,
                None,
            )
            .unwrap();
        assert_eq!(updated.title, "New Task 1 Title");
        assert_eq!(updated.status, KanbanStatus::NeedReview);
        assert_eq!(state.items_in_column(0).len(), 0);
        assert_eq!(state.items_in_column(2).len(), 1);

        // 3. Clear dead terminals helper
        let term_id = "term-123".to_string();
        state.update_item(&item2.uuid, None, None, None, Some(term_id.clone()), None);
        assert_eq!(state.items[1].terminal_id, Some(term_id.clone()));

        // Simulate terminal alive check (returns false for term-123)
        let any_cleared = state.clear_dead_terminals(|tid| tid != "term-123");
        assert!(any_cleared);
        assert_eq!(state.items[1].terminal_id, None);

        // 4. Delete item
        let deleted = state.delete_item(&item1.uuid).unwrap();
        assert_eq!(deleted.uuid, item1.uuid);
        assert_eq!(state.items.len(), 1);
    }

    #[test]
    fn test_kanban_navigation_and_shift() {
        let mut state = KanbanState::default();
        let _item1 = state.add_item("Task 1".into(), None, Some(KanbanStatus::Todo), None);
        let _item2 = state.add_item("Task 2".into(), None, Some(KanbanStatus::Todo), None);

        // Move left at column 0 should do nothing
        state.move_col_left();
        assert_eq!(state.selected_col, 0);

        // Move row down
        state.move_row_down();
        assert_eq!(state.selected_row, 1);

        // Shift item right from Todo to InProgress
        state.shift_item_right();
        assert_eq!(state.selected_col, 1);
        assert_eq!(state.selected_row, 0); // moves to end of next column (only 1 item there)
        assert_eq!(state.items_in_column(1).len(), 1);
        assert_eq!(state.items_in_column(0).len(), 1);
    }
}
