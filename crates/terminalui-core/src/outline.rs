//! Compact one-line-per-node outline for agents and logs.

use crate::node::Node;

const VALUE_LIMIT: usize = 48;

/// Quote and escape a value for the outline, truncating it to keep lines short.
pub fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for (index, character) in value.chars().enumerate() {
        if index >= VALUE_LIMIT {
            out.push_str("...");
            break;
        }
        match character {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.extend(character.escape_debug()),
        }
    }
    out.push('\'');
    out
}

pub(crate) fn tree_outline(root: &Node, focus: Option<&str>) -> String {
    let mut out = String::new();
    match focus {
        Some(focus) => {
            out.push_str("focus=");
            out.push_str(focus);
        }
        None => out.push_str("focus=none"),
    }
    out.push('\n');
    write_node(root, "", true, true, &mut out);
    out.pop();
    out
}

fn write_node(node: &Node, prefix: &str, last: bool, root: bool, out: &mut String) {
    if root {
        out.push_str(&line(node));
    } else {
        out.push_str(prefix);
        out.push_str(if last { "└─ " } else { "├─ " });
        out.push_str(&line(node));
    }
    out.push('\n');
    let child_prefix = if root {
        String::new()
    } else {
        format!("{prefix}{}", if last { "   " } else { "│  " })
    };
    let children = node.view.children();
    for (index, child) in children.iter().enumerate() {
        let child_last = index + 1 == children.len();
        write_node(child, &child_prefix, child_last, false, out);
    }
}

fn line(node: &Node) -> String {
    let mut line = format!("{} [{}]", node.id, node.view.kind());
    if let Some(state) = node.view.outline() {
        line.push_str(" {");
        line.push_str(&state);
        line.push('}');
    }
    line
}
