// Allow dead code in src/kanban.rs when the kanban feature is disabled in the build.
// This preserves the pure data structure for snapshot/session compatibility.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::api::schema::{KanbanItem, KanbanStatus};

const KANBAN_COLUMN_COUNT: usize = 4;
const DESKTOP_CARD_HEIGHT: u16 = 4;
const DESKTOP_CARD_SPACING: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanbanBoardLayout {
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanbanBoardDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanbanBoardAction {
    None,
    Mutated,
    ActivateCard {
        uuid: String,
        terminal_id: Option<String>,
    },
    CopyUuid {
        uuid: String,
    },
}

#[derive(Debug, Clone)]
pub struct KanbanBoardProjection {
    pub columns: Vec<KanbanColumnProjection>,
}

#[derive(Debug, Clone)]
pub struct KanbanColumnProjection {
    pub index: usize,
    pub status: KanbanStatus,
    pub area: ratatui::layout::Rect,
    pub inner_area: ratatui::layout::Rect,
    pub item_count: usize,
    pub is_selected: bool,
    pub cards: Vec<KanbanCardProjection>,
}

#[derive(Debug, Clone)]
pub struct KanbanCardProjection {
    pub item: KanbanItem,
    pub row_index: usize,
    pub area: ratatui::layout::Rect,
    pub is_selected: bool,
}

impl KanbanBoardProjection {
    pub fn card_at(&self, x: u16, y: u16) -> Option<&KanbanCardProjection> {
        self.columns
            .iter()
            .flat_map(|column| column.cards.iter())
            .find(|card| rect_contains(card.area, x, y))
    }

    pub fn column_at(&self, layout: KanbanBoardLayout, x: u16, y: u16) -> Option<usize> {
        self.columns.iter().find_map(|column| {
            let inside = match layout {
                KanbanBoardLayout::Desktop => {
                    x >= column.area.x && x < column.area.x.saturating_add(column.area.width)
                }
                KanbanBoardLayout::Mobile => {
                    y >= column.area.y && y < column.area.y.saturating_add(column.area.height)
                }
            };
            inside.then_some(column.index)
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KanbanState {
    pub items: Vec<KanbanItem>,
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
    ) -> KanbanItem {
        let item = KanbanItem {
            uuid: uuid::Uuid::new_v4().to_string(),
            title,
            description: description.unwrap_or_default(),
            status: status.unwrap_or(KanbanStatus::Todo),
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
    ) -> Option<KanbanItem> {
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

    pub fn delete_item(&mut self, uuid: &str) -> Option<KanbanItem> {
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

    pub fn items_in_column(&self, col: usize) -> Vec<&KanbanItem> {
        let status = match col {
            0 => KanbanStatus::Todo,
            1 => KanbanStatus::InProgress,
            2 => KanbanStatus::NeedReview,
            3 => KanbanStatus::Done,
            _ => return vec![],
        };
        self.items
            .iter()
            .filter(|item| item.status == status)
            .collect()
    }

    pub fn board_projection(
        &self,
        area: ratatui::layout::Rect,
        layout: KanbanBoardLayout,
    ) -> KanbanBoardProjection {
        let sections = match layout {
            KanbanBoardLayout::Desktop => ratatui::layout::Layout::horizontal([
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
            ])
            .split(area),
            KanbanBoardLayout::Mobile => ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(25),
            ])
            .split(area),
        };

        let selected_col = self.clamped_selected_col();
        let selected_row = self.clamped_selected_row(selected_col);
        let mut columns = Vec::with_capacity(KANBAN_COLUMN_COUNT);
        for col_idx in 0..KANBAN_COLUMN_COUNT {
            let area = sections[col_idx];
            let inner_area = ratatui::layout::Rect::new(
                area.x.saturating_add(1),
                area.y.saturating_add(1),
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            );
            let items = self.items_in_column(col_idx);
            let item_count = items.len();
            let is_selected = selected_col == col_idx;
            let scroll_offset = board_scroll_offset(layout, inner_area, is_selected, selected_row);
            let max_visible_cards = max_visible_cards(layout, inner_area);
            let cards = items
                .iter()
                .skip(scroll_offset)
                .take(max_visible_cards)
                .enumerate()
                .filter_map(|(idx, item)| {
                    let row_index = scroll_offset + idx;
                    let card_area = card_area(layout, inner_area, idx)?;
                    Some(KanbanCardProjection {
                        item: (*item).clone(),
                        row_index,
                        area: card_area,
                        is_selected: is_selected && selected_row == row_index,
                    })
                })
                .collect();

            columns.push(KanbanColumnProjection {
                index: col_idx,
                status: status_for_column(col_idx).unwrap_or(KanbanStatus::Todo),
                area,
                inner_area,
                item_count,
                is_selected,
                cards,
            });
        }

        KanbanBoardProjection { columns }
    }

    pub fn clamp_board_selection(&mut self) {
        self.selected_col = self.clamped_selected_col();
        self.selected_row = self.clamped_selected_row(self.selected_col);
    }

    pub fn copy_selected_uuid(&self) -> KanbanBoardAction {
        self.selected_item()
            .map(|item| KanbanBoardAction::CopyUuid {
                uuid: item.uuid.clone(),
            })
            .unwrap_or(KanbanBoardAction::None)
    }

    pub fn open_selected_detail(&mut self) -> KanbanBoardAction {
        if let Some(uuid) = self.selected_item().map(|item| item.uuid.clone()) {
            self.set_detail_uuid(Some(uuid));
            KanbanBoardAction::Mutated
        } else {
            KanbanBoardAction::None
        }
    }

    pub fn activate_card_at(
        &mut self,
        area: ratatui::layout::Rect,
        layout: KanbanBoardLayout,
        x: u16,
        y: u16,
    ) -> KanbanBoardAction {
        let projection = self.board_projection(area, layout);
        let Some((col_idx, row_index, item)) = projection.card_at(x, y).map(|card| {
            (
                column_index_for_status(card.item.status),
                card.row_index,
                card.item.clone(),
            )
        }) else {
            return KanbanBoardAction::None;
        };
        self.selected_col = col_idx;
        self.selected_row = row_index;
        KanbanBoardAction::ActivateCard {
            uuid: item.uuid,
            terminal_id: item.terminal_id,
        }
    }

    pub fn open_card_at(
        &mut self,
        area: ratatui::layout::Rect,
        layout: KanbanBoardLayout,
        x: u16,
        y: u16,
    ) -> KanbanBoardAction {
        let projection = self.board_projection(area, layout);
        let Some((col_idx, row_index, uuid)) = projection.card_at(x, y).map(|card| {
            (
                column_index_for_status(card.item.status),
                card.row_index,
                card.item.uuid.clone(),
            )
        }) else {
            return KanbanBoardAction::None;
        };
        self.selected_col = col_idx;
        self.selected_row = row_index;
        self.set_detail_uuid(Some(uuid));
        KanbanBoardAction::Mutated
    }

    pub fn scroll_board_at(
        &mut self,
        area: ratatui::layout::Rect,
        layout: KanbanBoardLayout,
        x: u16,
        y: u16,
        delta: i16,
    ) -> KanbanBoardAction {
        let projection = self.board_projection(area, layout);
        let Some(col_idx) = projection.column_at(layout, x, y) else {
            return KanbanBoardAction::None;
        };
        if self.selected_col != col_idx {
            self.selected_col = col_idx;
            self.selected_row = 0;
        }

        let items_count = self.items_in_column(col_idx).len();
        if items_count == 0 {
            return KanbanBoardAction::None;
        }

        if delta < 0 {
            self.selected_row = self.selected_row.saturating_sub(1);
        } else if delta > 0 {
            self.selected_row = (self.selected_row + 1).min(items_count - 1);
        }
        KanbanBoardAction::Mutated
    }

    pub fn move_board_selection(
        &mut self,
        layout: KanbanBoardLayout,
        direction: KanbanBoardDirection,
    ) {
        match (layout, direction) {
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Left)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Up) => self.move_col_left(),
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Right)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Down) => self.move_col_right(),
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Up)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Left) => self.move_row_up(),
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Down)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Right) => self.move_row_down(),
        }
    }

    pub fn shift_selected_item_for_layout(
        &mut self,
        layout: KanbanBoardLayout,
        direction: KanbanBoardDirection,
    ) -> bool {
        let before = self
            .selected_item()
            .map(|item| (item.uuid.clone(), item.status));
        match (layout, direction) {
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Left)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Up) => self.shift_item_left(),
            (KanbanBoardLayout::Desktop, KanbanBoardDirection::Right)
            | (KanbanBoardLayout::Mobile, KanbanBoardDirection::Down) => self.shift_item_right(),
            _ => {}
        }
        before
            != self
                .selected_item()
                .map(|item| (item.uuid.clone(), item.status))
    }

    pub fn move_col_left(&mut self) {
        if self.selected_col > 0 {
            self.selected_col -= 1;
            self.selected_row = 0;
        }
    }

    fn selected_item(&self) -> Option<&KanbanItem> {
        self.items_in_column(self.clamped_selected_col())
            .get(self.clamped_selected_row(self.clamped_selected_col()))
            .copied()
    }

    fn clamped_selected_col(&self) -> usize {
        self.selected_col.min(KANBAN_COLUMN_COUNT - 1)
    }

    fn clamped_selected_row(&self, col: usize) -> usize {
        self.selected_row
            .min(self.items_in_column(col).len().saturating_sub(1))
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

pub fn status_for_column(col: usize) -> Option<KanbanStatus> {
    match col {
        0 => Some(KanbanStatus::Todo),
        1 => Some(KanbanStatus::InProgress),
        2 => Some(KanbanStatus::NeedReview),
        3 => Some(KanbanStatus::Done),
        _ => None,
    }
}

pub fn column_index_for_status(status: KanbanStatus) -> usize {
    match status {
        KanbanStatus::Todo => 0,
        KanbanStatus::InProgress => 1,
        KanbanStatus::NeedReview => 2,
        KanbanStatus::Done => 3,
    }
}

fn max_visible_cards(layout: KanbanBoardLayout, inner_area: ratatui::layout::Rect) -> usize {
    match layout {
        KanbanBoardLayout::Desktop => inner_area
            .height
            .checked_div(DESKTOP_CARD_HEIGHT + DESKTOP_CARD_SPACING)
            .map(usize::from)
            .unwrap_or(0),
        KanbanBoardLayout::Mobile => usize::from(inner_area.height > 0),
    }
}

fn board_scroll_offset(
    layout: KanbanBoardLayout,
    inner_area: ratatui::layout::Rect,
    is_selected_column: bool,
    selected_row: usize,
) -> usize {
    if !is_selected_column {
        return 0;
    }
    match layout {
        KanbanBoardLayout::Desktop => {
            let max_visible_cards = max_visible_cards(layout, inner_area);
            if max_visible_cards == 0 {
                0
            } else if selected_row >= max_visible_cards {
                selected_row - max_visible_cards + 1
            } else {
                0
            }
        }
        KanbanBoardLayout::Mobile => selected_row,
    }
}

fn card_area(
    layout: KanbanBoardLayout,
    inner_area: ratatui::layout::Rect,
    visible_index: usize,
) -> Option<ratatui::layout::Rect> {
    match layout {
        KanbanBoardLayout::Desktop => {
            let card_y = inner_area.y
                + (visible_index as u16 * (DESKTOP_CARD_HEIGHT + DESKTOP_CARD_SPACING));
            (card_y + DESKTOP_CARD_HEIGHT <= inner_area.y + inner_area.height).then_some(
                ratatui::layout::Rect::new(
                    inner_area.x,
                    card_y,
                    inner_area.width,
                    DESKTOP_CARD_HEIGHT,
                ),
            )
        }
        KanbanBoardLayout::Mobile => {
            let card_height = DESKTOP_CARD_HEIGHT.min(inner_area.height);
            (card_height > 0).then_some(ratatui::layout::Rect::new(
                inner_area.x,
                inner_area.y,
                inner_area.width,
                card_height,
            ))
        }
    }
}

fn rect_contains(rect: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::KanbanStatus;
    use ratatui::layout::Rect;

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

    #[test]
    fn board_projection_clamps_selection_and_exposes_visible_cards() {
        let mut state = KanbanState::default();
        let first = state.add_item("Task 1".into(), None, Some(KanbanStatus::Todo), None);
        let second = state.add_item("Task 2".into(), None, Some(KanbanStatus::Todo), None);
        state.selected_col = 9;
        state.selected_row = 9;

        let projection =
            state.board_projection(Rect::new(0, 0, 80, 20), KanbanBoardLayout::Desktop);

        assert!(projection.columns[3].is_selected);
        assert_eq!(projection.columns[0].cards.len(), 2);
        assert_eq!(projection.columns[0].cards[0].item.uuid, first.uuid);
        assert_eq!(projection.columns[0].cards[1].item.uuid, second.uuid);
    }

    #[test]
    fn board_projection_scrolls_selected_desktop_column() {
        let mut state = KanbanState::default();
        for idx in 0..5 {
            state.add_item(format!("Task {idx}"), None, Some(KanbanStatus::Todo), None);
        }
        state.selected_col = 0;
        state.selected_row = 4;

        let projection =
            state.board_projection(Rect::new(0, 0, 80, 20), KanbanBoardLayout::Desktop);

        assert_eq!(projection.columns[0].cards.len(), 3);
        assert_eq!(projection.columns[0].cards[0].row_index, 2);
        assert!(projection.columns[0].cards[2].is_selected);
    }

    #[test]
    fn board_projection_hit_testing_uses_visible_cards() {
        let mut state = KanbanState::default();
        let item = state.add_item(
            "Tracked".into(),
            None,
            Some(KanbanStatus::InProgress),
            Some("term-1".into()),
        );

        let action =
            state.activate_card_at(Rect::new(0, 0, 80, 20), KanbanBoardLayout::Desktop, 21, 1);

        assert_eq!(state.selected_col, 1);
        assert_eq!(state.selected_row, 0);
        assert_eq!(
            action,
            KanbanBoardAction::ActivateCard {
                uuid: item.uuid,
                terminal_id: Some("term-1".into()),
            }
        );
    }
}
