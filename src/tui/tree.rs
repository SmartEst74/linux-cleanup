use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

pub struct FileTree<'a> {
    pub nodes: &'a [FileTreeNode],
    pub selected: usize,
    pub title: String,
}

impl<'a> FileTree<'a> {
    pub fn new(title: &str, nodes: &'a [FileTreeNode]) -> Self {
        Self {
            nodes,
            selected: 0,
            title: title.to_string(),
        }
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        self
    }
}

fn format_size(bytes: u64) -> String {
    if bytes > 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes > 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

impl Widget for FileTree<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        // Title
        buf.set_string(
            area.x + 1,
            area.y,
            &self.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let visible_height = (area.height as usize).saturating_sub(2);

        for (i, node) in self.nodes.iter().enumerate().take(visible_height) {
            let y = area.y + 1 + i as u16;
            if y >= area.bottom().saturating_sub(1) {
                break;
            }

            let indent = "  ".repeat(node.depth);
            let icon = if node.is_dir { "+" } else { "-" };
            let size_str = format_size(node.size);

            let style = if i == self.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if node.is_dir {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            };

            let expand_marker = if node.is_dir {
                if node.expanded {
                    "[-]"
                } else {
                    "[+]"
                }
            } else {
                "   "
            };

            let line = format!(
                "{}{}{} {:>10}  {}",
                indent, expand_marker, icon, size_str, node.name
            );
            let truncated = if line.len() > area.width as usize {
                format!("{}...", &line[..area.width as usize - 3])
            } else {
                line
            };

            buf.set_string(area.x + 1, y, truncated, style);
        }
    }
}
