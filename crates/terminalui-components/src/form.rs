//! Form: validated label/input pairs that collect and hold form state.
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use serde::Serialize;
use std::any::Any;

use terminalui_core::input::InputEvent;
use terminalui_core::outline::quoted;
use terminalui_core::theme::Theme;
use terminalui_core::{node, Context, Node, Outcome, Text, TextStyle, View};

use super::input::Input;

const ERROR_SLOT: &str = "error";

/// Per-instance overrides for a form. Defaults match the theme.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FormStyle {
    /// Rows between fields. Default 1.
    pub field_gap: Option<u16>,
}

/// A single validated field. Its children are composed: a [`Text`] label and
/// an [`Input`] control, with an optional error [`Text`] inserted at render
/// time when validation fails.
#[derive(Serialize)]
pub struct FormField {
    /// Key used in the `Submitted` payload.
    pub name: String,
    /// Whether validation requires a non-empty value.
    pub required: bool,
    /// Custom message shown when a required field is left empty.
    pub required_message: Option<String>,
    children: Vec<Node>,
}

impl FormField {
    /// A text field. The label and the control are composed views.
    pub fn text(name: impl Into<String>, label: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(
            name.clone(),
            vec![
                node(
                    format!("{name}:label"),
                    Text::new(label).with_style(TextStyle::default().bold()),
                ),
                node(format!("{name}:input"), Input::new()),
            ],
        )
    }

    /// Compose a field from arbitrary views. The children render top to
    /// bottom: use [`FormField::text`] for the common label-plus-input case.
    pub fn new(name: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            name: name.into(),
            required: false,
            required_message: None,
            children,
        }
    }

    /// Sets the placeholder text on every composed [`Input`] child.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        let placeholder = placeholder.into();
        for child in &mut self.children {
            if let Some(input) = child.view.as_any().downcast_mut::<Input>() {
                input.set_placeholder(placeholder.clone());
            }
        }
        self
    }

    /// Marks the field as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Marks the field required and sets a custom validation message.
    pub fn required_with_message(mut self, message: impl Into<String>) -> Self {
        self.required = true;
        self.required_message = Some(message.into());
        self
    }

    /// The control's current value.
    pub fn value(&self) -> String {
        self.children
            .iter()
            .filter_map(|child| child.view.as_any_ref().downcast_ref::<Input>())
            .next()
            .map(|input| input.state.value.clone())
            .unwrap_or_default()
    }

    /// Show or clear the error message row.
    pub fn set_error(&mut self, error: Option<impl Into<String>>) {
        self.children.retain(|child| child.id != ERROR_SLOT);
        if let Some(error) = error {
            self.children.push(node(ERROR_SLOT, Text::new(error)));
        }
    }

    fn is_empty(&self) -> bool {
        self.value().trim().is_empty()
    }
}

impl View for FormField {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "form_field"
    }

    fn outline(&self) -> Option<String> {
        let message = self
            .children
            .iter()
            .find(|child| child.id == ERROR_SLOT)
            .and_then(|child| child.view.as_any_ref().downcast_ref::<Text>())
            .map(Text::text)
            .map(quoted)
            .unwrap_or_else(|| "none".to_string());
        Some(format!(
            "name={}, required={}, error={}",
            quoted(&self.name),
            self.required,
            message
        ))
    }

    fn measure(&self) -> Option<(u16, u16)> {
        let mut width = 0u16;
        let mut height = 0u16;
        for child in &self.children {
            let (w, h) = child.view.measure().unwrap_or((0, 1));
            width = width.max(w);
            height += h;
        }
        Some((width, height.max(1)))
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let has_label = self
            .children
            .first()
            .map(|child| child.view.as_any_ref().is::<Text>())
            .unwrap_or(false);
        let required_marker = theme.destructive;
        if area.height == 0 {
            return;
        }
        let gap = 0u16;
        let mut y = area.y;
        let bottom = area.y + area.height;
        for (index, child) in self.children.iter_mut().enumerate() {
            if y >= bottom {
                break;
            }
            let height = child
                .view
                .measure()
                .map(|(_, height)| height.max(1))
                .unwrap_or(1)
                .min(bottom - y);
            let cell = Rect::new(area.x, y, area.width, height);
            if child.id == ERROR_SLOT {
                buf.set_style(cell, Style::default().fg(theme.destructive));
            }
            child.view.render(cell, buf, cx, theme);
            if has_label && index == 0 && self.required {
                let label_width = child.view.measure().map(|(width, _)| width).unwrap_or(0);
                let marker_x = cell.x + label_width + 1;
                if marker_x < cell.x + cell.width {
                    buf.set_string(marker_x, y, "*", Style::default().fg(required_marker));
                }
            }
            y = y.saturating_add(height + gap);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _cx: &mut Context) -> Outcome {
        Outcome::Ignored
    }
}

/// A stack of [`FormField`] views. Tab and BackTab move between fields
/// (handled by the tree). Enter submits.
#[derive(Serialize)]
pub struct Form {
    /// Per-instance overrides; defaults match the theme.
    pub style: FormStyle,
    children: Vec<Node>,
    submitted: Option<Vec<(String, String)>>,
}

impl Form {
    /// Convenience constructor: each `(name, label)` pair becomes a text field.
    pub fn with_fields(fields: Vec<(impl Into<String>, impl Into<String>)>) -> Self {
        let children = fields
            .into_iter()
            .map(|(name, label)| {
                let name = name.into();
                node(name.clone(), FormField::text(name, label))
            })
            .collect();
        Self::new(children)
    }

    /// Compose a form from [`FormField`] nodes.
    pub fn new(fields: Vec<Node>) -> Self {
        Self {
            style: FormStyle::default(),
            children: fields,
            submitted: None,
        }
    }

    /// Builder shorthand: replace the style.
    pub fn with_style(mut self, style: FormStyle) -> Self {
        self.style = style;
        self
    }

    /// Values in field order.
    pub fn values(&self) -> Vec<(String, String)> {
        self.children
            .iter()
            .filter_map(|child| {
                child
                    .view
                    .as_any_ref()
                    .downcast_ref::<FormField>()
                    .map(|field| (field.name.clone(), field.value()))
            })
            .collect()
    }

    /// Mark required fields empty and clear the error rows of the rest.
    /// Returns true when every field passed validation. On success the
    /// `(name, value)` pairs are buffered for [`Form::take_submitted`].
    pub fn validate(&mut self) -> bool {
        let mut valid = true;
        for child in &mut self.children {
            let Some(field) = child.view.as_any().downcast_mut::<FormField>() else {
                continue;
            };
            if field.required && field.is_empty() {
                let message = field
                    .required_message
                    .clone()
                    .unwrap_or_else(|| "required".to_string());
                field.set_error(Some(message));
                valid = false;
            } else {
                field.set_error(None::<String>);
            }
        }
        if valid {
            self.submitted = Some(self.values());
        }
        valid
    }

    /// Drains the submit outbox. Returns the values once per successful
    /// validation, from Enter, activation, or an explicit `validate` call.
    pub fn take_submitted(&mut self) -> Option<Vec<(String, String)>> {
        self.submitted.take()
    }

    fn field_gap(&self) -> u16 {
        self.style.field_gap.unwrap_or(1)
    }
}

impl View for Form {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn kind(&self) -> &'static str {
        "form"
    }

    fn children(&self) -> &[Node] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node] {
        &mut self.children
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, cx: &mut Context, theme: &Theme) {
        let gap = self.field_gap();
        let mut y = area.y;
        let bottom = area.y + area.height;
        for child in &mut self.children {
            if y >= bottom {
                break;
            }
            let height = child
                .view
                .measure()
                .map(|(_, height)| height)
                .unwrap_or(1)
                .min(bottom - y)
                .max(1);
            let cell = Rect::new(area.x, y, area.width, height);
            child.view.render(cell, buf, cx, theme);
            y = y.saturating_add(height + gap);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, _cx: &mut Context) -> Outcome {
        if let InputEvent::Key(key) = event {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                let _ = self.validate();
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    fn activate(&mut self, _cx: &Context) -> bool {
        self.validate();
        true
    }
}
