use crate::shape::AnnotationItem;

pub enum AnnotationCommand {
    Add(AnnotationItem),
    Remove(String, Option<AnnotationItem>),
    Clear(Vec<AnnotationItem>),
}

pub struct CommandStack {
    items: Vec<AnnotationItem>,
    undo_stack: Vec<AnnotationCommand>,
    redo_stack: Vec<AnnotationCommand>,
}

impl Default for CommandStack {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn items(&self) -> &[AnnotationItem] {
        &self.items
    }

    pub fn add_item(&mut self, item: AnnotationItem) {
        self.items.push(item.clone());
        self.undo_stack.push(AnnotationCommand::Add(item));
        self.redo_stack.clear();
    }

    pub fn remove_item(&mut self, id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|it| it.id == id) {
            let removed = self.items.remove(pos);
            self.undo_stack
                .push(AnnotationCommand::Remove(id.to_string(), Some(removed)));
            self.redo_stack.clear();
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let old = std::mem::take(&mut self.items);
        self.undo_stack.push(AnnotationCommand::Clear(old));
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let cmd = match self.undo_stack.pop() {
            Some(c) => c,
            None => return false,
        };

        match cmd {
            AnnotationCommand::Add(item) => {
                if let Some(pos) = self.items.iter().position(|it| it.id == item.id) {
                    let removed = self.items.remove(pos);
                    self.redo_stack.push(AnnotationCommand::Add(removed));
                }
            }
            AnnotationCommand::Remove(id, Some(item)) => {
                self.items.push(item.clone());
                self.redo_stack
                    .push(AnnotationCommand::Remove(id, Some(item)));
            }
            AnnotationCommand::Remove(_, None) => {}
            AnnotationCommand::Clear(old_items) => {
                self.redo_stack
                    .push(AnnotationCommand::Clear(self.items.clone()));
                self.items = old_items;
            }
        }
        true
    }

    pub fn redo(&mut self) -> bool {
        let cmd = match self.redo_stack.pop() {
            Some(c) => c,
            None => return false,
        };

        match cmd {
            AnnotationCommand::Add(item) => {
                self.items.push(item.clone());
                self.undo_stack.push(AnnotationCommand::Add(item));
            }
            AnnotationCommand::Remove(id, Some(_)) => {
                if let Some(pos) = self.items.iter().position(|it| it.id == id) {
                    let removed = self.items.remove(pos);
                    self.undo_stack
                        .push(AnnotationCommand::Remove(id, Some(removed)));
                }
            }
            AnnotationCommand::Remove(_, None) => {}
            AnnotationCommand::Clear(items_to_clear) => {
                self.undo_stack
                    .push(AnnotationCommand::Clear(self.items.clone()));
                self.items.clear();
                let _ = items_to_clear;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{AnnotationKind, RectShape};
    use domain::ColorRgba;

    #[test]
    fn test_undo_redo() {
        let mut stack = CommandStack::new();
        let rect = AnnotationItem::new(AnnotationKind::Rect(RectShape {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
            stroke_color: ColorRgba::rgb(255, 0, 0),
            stroke_width: 2.0,
            fill_color: None,
        }));

        stack.add_item(rect);
        assert_eq!(stack.items().len(), 1);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo();
        assert_eq!(stack.items().len(), 0);
        assert!(stack.can_redo());

        stack.redo();
        assert_eq!(stack.items().len(), 1);
    }
}
