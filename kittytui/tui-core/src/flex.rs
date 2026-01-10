use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::Renderable;

/// A spacer that takes up a fixed number of rows
pub struct Spacer(pub u16);

impl Renderable for Spacer {
    fn render(&self, _area: Rect, _buf: &mut Buffer) {}

    fn height(&self, _width: u16) -> u16 {
        self.0
    }
}

/// An item that can be either borrowed or owned
pub enum RenderableItem<'a> {
    Borrowed(&'a dyn Renderable),
    Owned(Box<dyn Renderable + 'a>),
}

impl<'a> RenderableItem<'a> {
    pub fn borrowed(r: &'a dyn Renderable) -> Self {
        Self::Borrowed(r)
    }

    pub fn owned(r: impl Renderable + 'static) -> Self {
        Self::Owned(Box::new(r))
    }
}

impl Renderable for RenderableItem<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Borrowed(r) => r.render(area, buf),
            Self::Owned(r) => r.render(area, buf),
        }
    }

    fn height(&self, width: u16) -> u16 {
        match self {
            Self::Borrowed(r) => r.height(width),
            Self::Owned(r) => r.height(width),
        }
    }
}

pub struct FlexItem<'a> {
    pub flex: u16,
    pub item: RenderableItem<'a>,
}

pub struct FlexRenderable<'a> {
    items: Vec<FlexItem<'a>>,
}

impl<'a> FlexRenderable<'a> {
    pub fn vertical() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, flex: u16, renderable: &'a dyn Renderable) {
        self.items.push(FlexItem {
            flex,
            item: RenderableItem::Borrowed(renderable),
        });
    }

    pub fn push_owned(&mut self, flex: u16, renderable: impl Renderable + 'a) {
        self.items.push(FlexItem {
            flex,
            item: RenderableItem::Owned(Box::new(renderable)),
        });
    }

    pub fn push_spacer(&mut self, rows: u16) {
        self.push_owned(0, Spacer(rows));
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Renderable for FlexRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || self.items.is_empty() {
            return;
        }

        let width = area.width;
        let total_height = area.height;

        let fixed_height: u16 = self
            .items
            .iter()
            .filter(|item| item.flex == 0)
            .map(|item| item.item.height(width))
            .sum();

        let total_flex: u16 = self.items.iter().map(|item| item.flex).sum();

        let remaining_height = total_height.saturating_sub(fixed_height);

        let mut y = area.y;

        for item in &self.items {
            let item_height = if item.flex == 0 {
                item.item.height(width)
            } else if total_flex > 0 {
                (remaining_height as u32 * item.flex as u32 / total_flex as u32) as u16
            } else {
                0
            };

            if item_height > 0 && y < area.y + total_height {
                let available_height = (area.y + total_height).saturating_sub(y);
                let height = item_height.min(available_height);

                let item_area = Rect {
                    x: area.x,
                    y,
                    width,
                    height,
                };

                item.item.render(item_area, buf);
                y += height;
            }
        }
    }

    fn height(&self, width: u16) -> u16 {
        self.items
            .iter()
            .map(|item| {
                if item.flex == 0 {
                    item.item.height(width)
                } else {
                    item.flex
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedHeight(u16);

    impl Renderable for FixedHeight {
        fn render(&self, _area: Rect, _buf: &mut Buffer) {}
        fn height(&self, _width: u16) -> u16 {
            self.0
        }
    }

    #[test]
    fn test_flex_empty() {
        let flex = FlexRenderable::vertical();
        assert!(flex.is_empty());
        assert_eq!(flex.height(80), 0);
    }

    #[test]
    fn test_flex_fixed_items() {
        let item1 = FixedHeight(2);
        let item2 = FixedHeight(3);

        let mut flex = FlexRenderable::vertical();
        flex.push(0, &item1);
        flex.push(0, &item2);

        assert_eq!(flex.height(80), 5);
    }

    #[test]
    fn test_flex_mixed_items() {
        let fixed = FixedHeight(2);
        let flexible = FixedHeight(1);

        let mut flex = FlexRenderable::vertical();
        flex.push(0, &fixed);
        flex.push(1, &flexible);

        assert_eq!(flex.height(80), 3);
    }

    #[test]
    fn test_flex_with_spacer() {
        let item = FixedHeight(2);

        let mut flex = FlexRenderable::vertical();
        flex.push(0, &item);
        flex.push_spacer(1);
        flex.push_owned(0, FixedHeight(3));

        assert_eq!(flex.height(80), 6);
    }
}
