use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct PieChart {
    pub data: Vec<(String, u64)>,
    pub title: String,
}

impl PieChart {
    pub fn new(title: &str) -> Self {
        Self {
            data: Vec::new(),
            title: title.to_string(),
        }
    }

    pub fn data(mut self, data: Vec<(String, u64)>) -> Self {
        self.data = data;
        self
    }
}

const COLORS: &[Color] = &[
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::Gray,
];

impl Widget for PieChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 || self.data.is_empty() {
            return;
        }

        let total: u64 = self.data.iter().map(|(_, v)| v).sum();
        if total == 0 {
            return;
        }

        // Draw title
        let title_x = area.x + (area.width.saturating_sub(self.title.len() as u16)) / 2;
        buf.set_string(title_x, area.y, &self.title, Style::default());

        let chart_top = area.y + 1;
        let chart_height = (area.height.saturating_sub(4)) as usize;
        let chart_width = area.width as usize;

        if chart_height < 3 || chart_width < 10 {
            return;
        }

        let center_x = chart_width / 2;
        let center_y = chart_height / 2;
        let radius_x = (chart_width / 2).saturating_sub(1);
        let radius_y = chart_height / 2;

        // Build percentage breakpoints for filling
        let mut cumulative: Vec<f64> = Vec::new();
        let mut sum = 0.0;
        for (_, val) in &self.data {
            sum += *val as f64 / total as f64;
            cumulative.push(sum);
        }

        // Render using filled block characters
        for row in 0..chart_height {
            for col in 0..chart_width {
                let dy = row as f64 - center_y as f64;
                let dx = col as f64 - center_x as f64;

                // Normalize to unit circle
                let ny = if radius_y > 0 {
                    dy / radius_y as f64
                } else {
                    0.0
                };
                let nx = if radius_x > 0 {
                    dx / radius_x as f64
                } else {
                    0.0
                };

                if nx * nx + ny * ny <= 1.0 {
                    // Calculate angle (0 to 2*PI)
                    let angle = (-ny).atan2(nx);
                    let normalized = if angle < 0.0 {
                        angle + 2.0 * std::f64::consts::PI
                    } else {
                        angle
                    };
                    let pct = normalized / (2.0 * std::f64::consts::PI);

                    // Find which segment this belongs to
                    let segment = cumulative
                        .iter()
                        .position(|&c| pct <= c)
                        .unwrap_or(self.data.len() - 1);

                    let color = COLORS[segment % COLORS.len()];
                    let x = area.x + col as u16;
                    let y = chart_top + row as u16;

                    if x < area.right() && y < area.bottom() {
                        buf[(x, y)]
                            .set_symbol("█")
                            .set_style(Style::default().fg(color));
                    }
                }
            }
        }

        // Draw legend at the bottom
        let legend_y = area.bottom().saturating_sub(2);
        let mut legend_x = area.x + 1;
        for (i, (name, val)) in self.data.iter().enumerate().take(8) {
            let pct = (*val as f64 / total as f64 * 100.0) as u64;
            let label = format!("{} {}%", &name[..name.len().min(10)], pct);
            let color = COLORS[i % COLORS.len()];

            if legend_x + label.len() as u16 + 2 < area.right() {
                buf[(legend_x, legend_y)]
                    .set_symbol("●")
                    .set_style(Style::default().fg(color));
                buf.set_string(legend_x + 2, legend_y, &label, Style::default());
                legend_x += label.len() as u16 + 4;
            }
        }
    }
}
