//! A horizontal hairline drawn across its area.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use serde::Serialize;

use terminalui_core::input::InputEvent;
use terminalui_core::theme::Theme;
use terminalui_core::{Context, Outcome, View};

use super::chrome::draw_box;
use super::chrome::Chrome;
use super::presentation::RenderStyle;

/// A one-cell-high rule. The glyph resolves from [`RenderStyle::hairline`]
/// unless [`Separator::with_glyph`] pins one explicitly.
///
/// With [`Separator::with_chrome`] the line is drawn as a one-cell-high
/// boxed border instead: radius, dashes, double strokes, all from the
/// shared chrome machinery.
#[derive(Serialize)]
pub struct Separator {
    presentation: RenderStyle,
    glyph: Option<String>,
    chrome: Option<Chrome>,
}

impl Separator {
    pub fn new() -> Self {
        Self {
            presentation: RenderStyle::default(),
            glyph: None,
            chrome: None,
        }
    }

    /// Pin the stroke glyph instead of resolving it from the presentation.
    pub fn with_glyph(mut self, glyph: char) -> Self {
        self.glyph = Some(glyph.to_string());
        self
    }

    pub fn with_presentation(mut self, presentation: RenderStyle) -> Self {
        self.presentation = presentation;
        self
    }

    /// Draw the rule as a one-cell-high chrome box (radius, dashes, double
    /// strokes) instead of a flat hairline.
    pub fn with_chrome(mut self, chrome: Chrome) -> Self {
        self.chrome = Some(chrome);
        self
    }

    fn stroke(&self) -> String {
        match &self.glyph {
            Some(glyph) => glyph.clone(),
            None => self.presentation.hairline().0.to_string(),
        }
    }
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Separator {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn kind(&self) -> &'static str {
        "separator"
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _cx: &mut Context, theme: &Theme) {
        if let Some(chrome) = &self.chrome {
            let _ = draw_box(buf, area, chrome, theme);
            return;
        }
        let stroke = self.stroke();
        let style = Style::default().fg(theme.border);
        for x in area.x..area.x + area.width {
            buf.set_string(x, area.y, &stroke, style);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}
