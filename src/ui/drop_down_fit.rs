//! Keeps an `iced_aw::DropDown` list under its field inside a scrollable.
//!
//! The drop-down keeps its list on screen by testing it against the viewport
//! it is handed, but reads only that viewport's width and height, as if it
//! started at the window's top-left corner. Inside a scrollable the viewport
//! is the scrollable's visible area in window coordinates, so a field in a
//! dock pallet (which does not start at the corner) sees its list "overflow",
//! and the list jumps left by its own width or up over the field. Wrapped
//! here, the drop-down gets a viewport reaching from the window corner to the
//! visible area's far edges: the test then trips only where the list really
//! leaves the visible area.
//!
//! The list itself opens against the window, not the scroll area: below the
//! field when it fits there, else above when that fits, else on the larger
//! side, cut to it. A list in a small window (Layer Manager colour cells)
//! was clipped by the scroll area's bottom or flipped over its own field.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, tree, Tree, Widget};
use iced::advanced::{mouse, overlay, renderer, Shell};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

use crate::app::Message;

/// Wrap a drop-down so its list is placed against the visible area's real
/// edges.
pub fn drop_down_fit<'a>(drop_down: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(DropDownFit {
        content: drop_down.into(),
        start: false,
    })
}

/// [`drop_down_fit`] with the list starting at its field's left edge
/// (kept inside the window): a list wider than its field otherwise opens
/// centred under it, reaching out left of it.
pub fn drop_down_fit_start<'a>(drop_down: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(DropDownFit {
        content: drop_down.into(),
        start: true,
    })
}

struct DropDownFit<'a> {
    content: Element<'a, Message>,
    /// Start the list at the field's left edge.
    start: bool,
}

impl Widget<Message, Theme, Renderer> for DropDownFit<'_> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn diff(&mut self, tree: &mut Tree) {
        self.content.as_widget_mut().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // No vertical flip inside the drop-down: FitOverlay picks the side.
        let from_corner = Rectangle {
            x: 0.0,
            y: 0.0,
            width: viewport.x + viewport.width,
            height: f32::MAX,
        };
        let start_x = self.start.then(|| layout.bounds().x + translation.x);
        let field_top = layout.bounds().y + translation.y;
        let field_bottom = field_top + layout.bounds().height;
        let inner = self.content.as_widget_mut().overlay(
            tree,
            layout,
            renderer,
            &from_corner,
            translation,
        )?;
        Some(overlay::Element::new(Box::new(FitOverlay {
            inner,
            start_x,
            field_top,
            field_bottom,
        })))
    }
}

struct FitOverlay<'a> {
    inner: overlay::Element<'a, Message, Theme, Renderer>,
    /// Where the list starts when it starts at its field's left edge.
    start_x: Option<f32>,
    /// The field's top and bottom edge in window coordinates.
    field_top: f32,
    field_bottom: f32,
}

/// The list's gap to its field (the drop-downs here use `.offset(2.0)`).
const GAP: f32 = 2.0;

/// Which side of the field a list of natural height `wanted` opens on, given
/// the window room `above` and `below` it: below when it fits, else above when
/// that fits, else the larger side. `true` = above.
fn opens_above(wanted: f32, above: f32, below: f32) -> bool {
    wanted > below && (wanted <= above || above > below)
}

impl overlay::Overlay<Message, Theme, Renderer> for FitOverlay<'_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let below = (bounds.height - self.field_bottom - GAP).max(0.0);
        let above = (self.field_top - GAP).max(0.0);
        // The list's natural height: laid out with room to spare below.
        let tall = Size::new(bounds.width, self.field_bottom + GAP + bounds.height.max(1.0) * 4.0);
        let wanted = self.inner.as_overlay_mut().layout(renderer, tall).bounds().height;
        let node = if opens_above(wanted, above, below) {
            // Lay out with exactly `above` of room below the field, then move it up.
            let fake = Size::new(bounds.width, self.field_bottom + GAP + above);
            let node = self.inner.as_overlay_mut().layout(renderer, fake);
            let h = node.bounds().height;
            let x = node.bounds().x;
            node.move_to(iced::Point::new(x, (self.field_top - GAP - h).max(0.0)))
        } else {
            self.inner.as_overlay_mut().layout(renderer, bounds)
        };
        match self.start_x {
            Some(x) => {
                let placed = node.bounds();
                let x = x.min(bounds.width - placed.width).max(0.0);
                node.move_to(iced::Point::new(x, placed.y))
            }
            None => node,
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.inner
            .as_overlay()
            .draw(renderer, theme, style, layout, cursor);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        self.inner
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, shell);
        if matches!(
            event,
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                ..
            })
        ) {
            crate::ui::dropdown_list::mark_dropdown_escape_captured();
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.inner
            .as_overlay()
            .mouse_interaction(layout, cursor, renderer)
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.inner
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::opens_above;

    #[test]
    fn list_opens_below_when_it_fits_else_above_else_on_the_larger_side() {
        assert!(!opens_above(200.0, 500.0, 250.0), "fits below");
        assert!(opens_above(200.0, 300.0, 120.0), "fits only above");
        assert!(opens_above(400.0, 300.0, 120.0), "fits nowhere, more room above");
        assert!(!opens_above(400.0, 100.0, 300.0), "fits nowhere, more room below");
    }
}
