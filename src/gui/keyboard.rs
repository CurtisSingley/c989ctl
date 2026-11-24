use crate::layout::{KEYS, KeyInfo, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use crate::protocol::{RgbMode, parse_color};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, alignment, border};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardViewMode {
    Layout,
    Lighting,
    WheelOrMacro,
}

pub struct KeyboardCanvas<'a, Message> {
    pub view_mode: KeyboardViewMode,
    pub selected_key: Option<&'static str>,
    pub active_bindings: &'a BTreeMap<String, String>,
    pub rgb_mode: RgbMode,
    pub rgb_color: (u8, u8, u8),
    pub rgb_full_spectrum: bool,
    pub per_key_colors: &'a BTreeMap<String, String>,
    pub on_left_click: Box<dyn Fn(&'static str) -> Message + 'a>,
    pub on_right_click: Box<dyn Fn(&'static str) -> Message + 'a>,
}

#[derive(Default)]
pub struct KeyboardCanvasState {
    hovered: Option<&'static str>,
}

impl<'a, Message: 'a> KeyboardCanvas<'a, Message> {
    pub fn view(self) -> Element<'a, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fixed(275.0))
            .into()
    }

    fn compute_transform(bounds: Rectangle) -> (f32, f32, f32) {
        let pad = 10.0;
        let avail_w = (bounds.width - pad * 2.0).max(100.0);
        let avail_h = (bounds.height - pad * 2.0).max(60.0);
        let scale = (avail_w / LAYOUT_WIDTH).min(avail_h / LAYOUT_HEIGHT);
        let drawn_w = LAYOUT_WIDTH * scale;
        let drawn_h = LAYOUT_HEIGHT * scale;
        let offset_x = (bounds.width - drawn_w) * 0.5;
        let offset_y = (bounds.height - drawn_h) * 0.5;
        (scale, offset_x, offset_y)
    }

    fn hit_test(bounds: Rectangle, position: Point) -> Option<&'static KeyInfo> {
        let (scale, ox, oy) = Self::compute_transform(bounds);
        let lx = (position.x - ox) / scale;
        let ly = (position.y - oy) / scale;
        KEYS.iter().find(|k| {
            let (rx, ry, rw, rh) = k.rect;
            lx >= rx && lx <= rx + rw && ly >= ry && ly <= ry + rh
        })
    }
}

fn short_action_label(action: &str) -> String {
    let norm = action.trim().to_lowercase();
    match norm.as_str() {
        "caps" | "capslock" => "Caps".into(),
        "backspace" | "bks" => "BKS".into(),
        "scroll_up" => "ScrUp".into(),
        "scroll_down" => "ScrDn".into(),
        "mouse_left" | "lclick" => "LClk".into(),
        "mouse_right" | "rclick" => "RClk".into(),
        "mouse_middle" | "mclick" => "MClk".into(),
        "disabled" | "none" => "Off".into(),
        "media_player" => "Media".into(),
        "play_pause" => "Play".into(),
        "prev_track" => "Prev".into(),
        "next_track" => "Next".into(),
        "vol_up" => "Vol+".into(),
        "vol_down" => "Vol-".into(),
        "mute" => "Mute".into(),
        "scrlk" => "ScrLk".into(),
        "insert" => "Ins".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        "left" => "←".into(),
        "right" => "→".into(),
        other => {
            if let Some(slot) = other.strip_prefix("macro:") {
                format!("M:{slot}")
            } else if other.len() > 6 {
                other[..6].to_string()
            } else {
                action.to_string()
            }
        }
    }
}

fn rgb_to_color((r, g, b): (u8, u8, u8)) -> Color {
    Color::from_rgb8(r, g, b)
}

fn is_bright((r, g, b): (u8, u8, u8)) -> bool {
    let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
    lum > 145.0
}

fn rainbow_color(t: f32) -> (u8, u8, u8) {
    let h = (t.fract() + 1.0).fract() * 6.0;
    let x = (1.0 - (h % 2.0 - 1.0).abs()) * 255.0;
    let (r, g, b) = match h as u32 {
        0 => (255.0, x, 0.0),
        1 => (x, 255.0, 0.0),
        2 => (0.0, 255.0, x),
        3 => (0.0, x, 255.0),
        4 => (x, 0.0, 255.0),
        _ => (255.0, 0.0, x),
    };
    (r as u8, g as u8, b as u8)
}

impl<'a, Message: 'a> canvas::Program<Message> for KeyboardCanvas<'a, Message> {
    type State = KeyboardCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let Some(pos) = cursor.position_in(bounds) else {
            state.hovered = None;
            return (canvas::event::Status::Ignored, None);
        };

        let hit = Self::hit_test(bounds, pos);
        state.hovered = hit.map(|k| k.name);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(k) = hit {
                    return (
                        canvas::event::Status::Captured,
                        Some((self.on_left_click)(k.name)),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(k) = hit {
                    return (
                        canvas::event::Status::Captured,
                        Some((self.on_right_click)(k.name)),
                    );
                }
            }
            _ => {}
        }

        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (scale, ox, oy) = Self::compute_transform(bounds);

        // Keyboard chassis background
        let bg = Path::rounded_rectangle(
            Point::new(ox - 8.0, oy - 8.0),
            Size::new(LAYOUT_WIDTH * scale + 16.0, LAYOUT_HEIGHT * scale + 16.0),
            border::Radius::from(10.0),
        );
        frame.fill(&bg, Color::from_rgb8(24, 26, 32));
        frame.stroke(
            &bg,
            Stroke::default()
                .with_color(Color::from_rgb8(52, 56, 68))
                .with_width(1.5),
        );

        for k in KEYS {
            let (rx, ry, rw, rh) = k.rect;
            let x = ox + rx * scale;
            let y = oy + ry * scale;
            let w = rw * scale;
            let h = rh * scale;

            let is_selected = self.selected_key == Some(k.name);
            let is_hovered = state.hovered == Some(k.name);
            let binding = self.active_bindings.get(k.name).map(|s| s.as_str());

            let (fill_color, text_color, sub_label) = match self.view_mode {
                KeyboardViewMode::Layout => {
                    if let Some(act) = binding {
                        let is_macro = act.to_lowercase().starts_with("macro:");
                        let fill = if is_macro {
                            Color::from_rgb8(22, 118, 130)
                        } else {
                            Color::from_rgb8(196, 94, 24)
                        };
                        (fill, Color::WHITE, Some(short_action_label(act)))
                    } else if !k.remappable {
                        (Color::from_rgb8(54, 38, 42), Color::from_rgb8(210, 170, 175), None)
                    } else {
                        (Color::from_rgb8(46, 50, 60), Color::from_rgb8(225, 228, 235), None)
                    }
                }
                KeyboardViewMode::Lighting => {
                    let rgb = match self.rgb_mode {
                        RgbMode::UserDefine => self
                            .per_key_colors
                            .get(k.name)
                            .and_then(|c| parse_color(c).ok())
                            .unwrap_or((36, 39, 48)),
                        RgbMode::Static => self.rgb_color,
                        _ if self.rgb_full_spectrum => rainbow_color(rx / LAYOUT_WIDTH),
                        _ => self.rgb_color,
                    };
                    let txt = if is_bright(rgb) {
                        Color::from_rgb8(18, 20, 24)
                    } else {
                        Color::from_rgb8(235, 238, 244)
                    };
                    (rgb_to_color(rgb), txt, None)
                }
                KeyboardViewMode::WheelOrMacro => {
                    if let Some(act) = binding {
                        if act.to_lowercase().starts_with("macro:") {
                            (
                                Color::from_rgb8(22, 132, 145),
                                Color::WHITE,
                                Some(short_action_label(act)),
                            )
                        } else {
                            (
                                Color::from_rgb8(64, 58, 52),
                                Color::from_rgb8(225, 215, 205),
                                Some(short_action_label(act)),
                            )
                        }
                    } else {
                        (Color::from_rgb8(44, 48, 58), Color::from_rgb8(180, 186, 198), None)
                    }
                }
            };

            let key_path = Path::rounded_rectangle(
                Point::new(x, y),
                Size::new(w, h),
                border::Radius::from((5.0 * scale).clamp(3.0, 7.0)),
            );

            let final_fill = if is_hovered {
                Color::from_rgba(
                    (fill_color.r + 0.10).min(1.0),
                    (fill_color.g + 0.10).min(1.0),
                    (fill_color.b + 0.10).min(1.0),
                    1.0,
                )
            } else {
                fill_color
            };
            frame.fill(&key_path, final_fill);

            let (stroke_color, stroke_w) = if is_selected {
                (Color::from_rgb8(56, 224, 245), 2.5)
            } else if !k.remappable && self.view_mode == KeyboardViewMode::Layout {
                (Color::from_rgb8(215, 58, 74), 1.6)
            } else if is_hovered {
                (Color::from_rgb8(150, 180, 210), 1.5)
            } else {
                (Color::from_rgb8(20, 22, 28), 1.0)
            };

            frame.stroke(
                &key_path,
                Stroke::default()
                    .with_color(stroke_color)
                    .with_width(stroke_w),
            );

            // Keycap label
            let base_font = (13.0 * scale).clamp(8.5, 13.0);
            if let Some(sub) = sub_label {
                frame.fill_text(Text {
                    content: k.label.to_string(),
                    position: Point::new(x + w * 0.5, y + h * 0.31),
                    color: Color::from_rgba(text_color.r, text_color.g, text_color.b, 0.78),
                    size: (base_font * 0.85).into(),
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Center,
                    ..Text::default()
                });
                frame.fill_text(Text {
                    content: sub,
                    position: Point::new(x + w * 0.5, y + h * 0.69),
                    color: text_color,
                    size: (base_font * 0.92).into(),
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Center,
                    ..Text::default()
                });
            } else {
                frame.fill_text(Text {
                    content: k.label.to_string(),
                    position: Point::new(x + w * 0.5, y + h * 0.5),
                    color: text_color,
                    size: base_font.into(),
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Center,
                    ..Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.hovered.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
