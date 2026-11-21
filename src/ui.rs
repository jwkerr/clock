use crate::clock::{ClockTime, Hand, HandDrag, direction, pointer_turn};
use eframe::egui::{
    self, Align2, CursorIcon, FontId, PointerButton, Sense, Shape, Stroke, Ui, Vec2, vec2,
};

fn hand_length(hand: Hand) -> f32 {
    match hand {
        Hand::Hour => 0.56,
        Hand::Minute => 0.87,
        Hand::Second => 0.94,
    }
}

fn hit_hand(offset: Vec2, radius: f32, time: ClockTime, show_seconds: bool) -> Option<Hand> {
    if offset.length() < 12.0 {
        return None;
    }
    [Hand::Hour, Hand::Minute, Hand::Second]
        .into_iter()
        .filter(|&h| h != Hand::Second || show_seconds)
        .filter_map(|h| {
            let (dx, dy) = direction(h.turn(time));
            let axis = vec2(dx, dy);
            let length = radius * hand_length(h);
            let closest = axis * offset.dot(axis).clamp(0.0, length);
            let distance = (offset - closest).length();
            let tip_distance = (offset - axis * length).length();
            (distance <= 16.0).then_some((h, distance + 0.04 * tip_distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(hand, _)| hand)
}

pub struct ClockApp {
    time: ClockTime,
    show_minutes: bool,
    show_arc: bool,
    show_seconds: bool,
    drag: Option<HandDrag>,
}

impl ClockApp {
    pub fn new(time: ClockTime) -> Self {
        Self {
            time,
            show_minutes: true,
            show_arc: false,
            show_seconds: false,
            drag: None,
        }
    }

    fn teacher_panel(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Teacher controls")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time (24-hour):");
                    time_field(ui, &mut self.time.hour, 23, "Hours");
                    ui.label(":");
                    time_field(ui, &mut self.time.minute, 59, "Minutes");
                    ui.label(":");
                    time_field(ui, &mut self.time.second, 59, "Seconds");
                });
                ui.checkbox(&mut self.show_minutes, "Show minutes in fives");
                ui.checkbox(&mut self.show_arc, "Show hour arc");
                if self.show_arc {
                    ui.horizontal(|ui| {
                        ui.colored_label(ui.visuals().hyperlink_color, "Elapsed");
                        ui.colored_label(ui.visuals().warn_fg_color, "Remaining");
                    });
                }
                ui.checkbox(&mut self.show_seconds, "Show seconds hand");
            });
    }

    fn handle_drag(&mut self, area: &egui::Response, radius: f32) {
        let center = area.rect.center();
        if area.drag_started_by(PointerButton::Primary) {
            self.drag = area
                .ctx
                .input(|i| i.pointer.press_origin())
                .and_then(|pos| {
                    let offset = pos - center;
                    let hand = hit_hand(offset, radius, self.time, self.show_seconds)?;
                    Some(HandDrag::new(
                        hand,
                        self.time,
                        pointer_turn(offset.x, offset.y),
                    ))
                });
        }
        if area.dragged_by(PointerButton::Primary) || area.drag_stopped_by(PointerButton::Primary) {
            if let (Some(drag), Some(pos)) = (&mut self.drag, area.interact_pointer_pos()) {
                let offset = pos - center;
                if offset.length() >= 12.0 {
                    let time = drag.move_to(pointer_turn(offset.x, offset.y));
                    if time != self.time {
                        self.time = time;
                        area.ctx.request_repaint();
                    }
                }
            }
        } else {
            self.drag = None;
        }
        if area.drag_stopped() {
            self.drag = None;
        }
        if self.drag.is_some() {
            area.ctx.set_cursor_icon(CursorIcon::Grabbing);
        } else if area
            .hover_pos()
            .is_some_and(|p| hit_hand(p - center, radius, self.time, self.show_seconds).is_some())
        {
            area.ctx.set_cursor_icon(CursorIcon::Grab);
        }
    }

    fn clock_face(&mut self, ui: &mut Ui) {
        let (area, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        let center = area.rect.center();
        
        let radius = area.rect.size().min_elem() * if self.show_minutes { 0.42 } else { 0.47 };
        if radius < 1.0 {
            return;
        }
        self.handle_drag(&area, radius);
        let point = |turn, length| {
            let (dx, dy) = direction(turn);
            center + vec2(dx, dy) * (radius * length)
        };
        let ink = ui.visuals().text_color();
        let accent = ui.visuals().hyperlink_color;
        painter.circle_stroke(center, radius, Stroke::new(1.5, ink));

        for tick in 0..60 {
            let turn = tick as f32 / 60.0;
            let major = tick % 5 == 0;
            painter.line_segment(
                [
                    point(turn, if major { 0.90 } else { 0.95 }),
                    point(turn, 1.0),
                ],
                Stroke::new(if major { 2.5 } else { 1.0 }, ink),
            );
            if major {
                let hour = if tick == 0 { 12 } else { tick / 5 };
                painter.text(
                    point(turn, 0.78),
                    Align2::CENTER_CENTER,
                    hour.to_string(),
                    FontId::proportional(radius * 0.14),
                    ink,
                );
                if self.show_minutes {
                    painter.text(
                        point(turn, 1.10),
                        Align2::CENTER_CENTER,
                        format!("{tick:02}"),
                        FontId::proportional(radius * 0.09),
                        ink,
                    );
                }
            }
        }

        let hour = self.time.hour_turn();
        let hour_length = hand_length(Hand::Hour);
        if self.show_arc {
            let start = self.time.hour_start_turn();
            for (from, to, color) in [
                (start, hour, accent),
                (hour, start + 1.0 / 12.0, ui.visuals().warn_fg_color),
            ] {
                if from < to {
                    let points = (0..=16)
                        .map(|i| point(from + (to - from) * i as f32 / 16.0, hour_length))
                        .collect();
                    painter.add(Shape::line(points, Stroke::new(radius * 0.022, color)));
                }
            }
        }

        painter.line_segment(
            [center, point(hour, hour_length)],
            Stroke::new(radius * 0.036, ink),
        );
        painter.line_segment(
            [
                center,
                point(self.time.minute_turn(), hand_length(Hand::Minute)),
            ],
            Stroke::new(radius * 0.021, ink),
        );
        if self.show_seconds {
            painter.line_segment(
                [
                    center,
                    point(self.time.second_turn(), hand_length(Hand::Second)),
                ],
                Stroke::new(radius * 0.009, accent),
            );
        }
        painter.circle_filled(center, radius * 0.033, ink);
    }
}

fn time_field(ui: &mut Ui, value: &mut u32, max: u32, name: &str) {
    ui.add(
        egui::DragValue::new(value)
            .range(0..=max)
            .custom_formatter(|value, _| format!("{:02}", value as u32)),
    )
    .on_hover_text(format!("{name}: drag to adjust or click to type"));
}

impl eframe::App for ClockApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.teacher_panel(ui);
            ui.separator();
            self.clock_face(ui);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_hands_can_be_grabbed_by_their_tips() {
        let time = ClockTime::new(0, 0, 0);
        for hand in [Hand::Hour, Hand::Minute, Hand::Second] {
            let offset = vec2(0.0, -200.0 * hand_length(hand));
            assert_eq!(hit_hand(offset, 200.0, time, true), Some(hand));
        }
        let time = ClockTime::new(3, 0, 30);
        assert_eq!(hit_hand(vec2(0.0, 185.0), 200.0, time, false), None);
        assert_eq!(hit_hand(Vec2::ZERO, 200.0, time, true), None);
    }

    #[test]
    fn pointer_drag_moves_the_grabbed_hand_until_release() {
        use egui::{Event, RawInput, Rect, pos2};
        let ctx = egui::Context::default();
        let mut app = ClockApp::new(ClockTime::new(3, 0, 0));
        let mut frame = |events| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 400.0))),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.clock_face(ui));
            output.textures_delta.clear();
            app.time
        };
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let top = pos2(200.0, 58.0);
        let right = pos2(342.0, 200.0);
        frame(vec![]);
        frame(vec![Event::PointerMoved(top), button(top, true)]);
        assert_eq!(
            frame(vec![Event::PointerMoved(right)]),
            ClockTime::new(3, 15, 0)
        );
        frame(vec![button(right, false)]);
        assert_eq!(
            frame(vec![Event::PointerMoved(top)]),
            ClockTime::new(3, 15, 0)
        );
    }
}
