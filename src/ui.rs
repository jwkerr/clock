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

fn hit_hand(offset: Vec2, radius: f32, turns: [f32; 3], show_seconds: bool) -> Option<Hand> {
    if offset.length() < 12.0 {
        return None;
    }
    Hand::ALL
        .into_iter()
        .filter(|&h| h != Hand::Second || show_seconds)
        .filter_map(|h| {
            let (dx, dy) = direction(turns[h as usize]);
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

enum Drag {
    Linked(HandDrag),
    Independent { hand: Hand, grab_offset: f32 },
}

pub struct ClockApp {
    time: ClockTime,
    independent_turns: Option<[f32; 3]>,
    show_minutes: bool,
    show_arc: bool,
    show_seconds: bool,
    drag: Option<Drag>,
}

impl ClockApp {
    pub fn new(time: ClockTime) -> Self {
        Self {
            time,
            independent_turns: None,
            show_minutes: true,
            show_arc: false,
            show_seconds: false,
            drag: None,
        }
    }

    fn hand_turns(&self) -> [f32; 3] {
        self.independent_turns
            .unwrap_or_else(|| Hand::ALL.map(|hand| hand.turn(self.time)))
    }

    fn set_independent(&mut self, independent: bool) {
        self.independent_turns = independent.then(|| self.hand_turns());
        self.drag = None;
    }

    fn teacher_panel(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Teacher controls")
            .default_open(true)
            .show(ui, |ui| {
                let mut independent = self.independent_turns.is_some();
                if ui.checkbox(&mut independent, "Independent hands").changed() {
                    self.set_independent(independent);
                }
                if !independent {
                    ui.horizontal(|ui| {
                        ui.label("Time (24-hour):");
                        time_field(ui, &mut self.time.hour, 23, "Hours");
                        ui.label(":");
                        time_field(ui, &mut self.time.minute, 59, "Minutes");
                        ui.label(":");
                        time_field(ui, &mut self.time.second, 59, "Seconds");
                    });
                    ui.checkbox(&mut self.show_arc, "Show hour arc");
                    if self.show_arc {
                        ui.horizontal(|ui| {
                            ui.colored_label(ui.visuals().hyperlink_color, "Elapsed");
                            ui.colored_label(ui.visuals().warn_fg_color, "Remaining");
                        });
                    }
                }
                ui.checkbox(&mut self.show_minutes, "Show minutes in fives");
                ui.checkbox(&mut self.show_seconds, "Show seconds hand");
            });
    }

    fn handle_drag(&mut self, area: &egui::Response, radius: f32) {
        let center = area.rect.center();
        if area.drag_started_by(PointerButton::Primary) {
            let turns = self.hand_turns();
            self.drag = area
                .ctx
                .input(|i| i.pointer.press_origin())
                .and_then(|pos| {
                    let offset = pos - center;
                    let hand = hit_hand(offset, radius, turns, self.show_seconds)?;
                    let pointer = pointer_turn(offset.x, offset.y);
                    Some(if self.independent_turns.is_some() {
                        Drag::Independent {
                            hand,
                            grab_offset: turns[hand as usize] - pointer,
                        }
                    } else {
                        Drag::Linked(HandDrag::new(hand, self.time, pointer))
                    })
                });
        }
        if area.dragged_by(PointerButton::Primary) || area.drag_stopped_by(PointerButton::Primary) {
            if let (Some(drag), Some(pos)) = (&mut self.drag, area.interact_pointer_pos()) {
                let offset = pos - center;
                if offset.length() >= 12.0 {
                    let pointer = pointer_turn(offset.x, offset.y);
                    match drag {
                        Drag::Linked(drag) => self.time = drag.move_to(pointer),
                        Drag::Independent { hand, grab_offset } => {
                            if let Some(turns) = &mut self.independent_turns {
                                turns[*hand as usize] = (pointer + *grab_offset).rem_euclid(1.0);
                            }
                        }
                    }
                    area.ctx.request_repaint();
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
        } else if area.hover_pos().is_some_and(|p| {
            hit_hand(p - center, radius, self.hand_turns(), self.show_seconds).is_some()
        }) {
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

        let [hour, minute, second] = self.hand_turns();
        let hour_length = hand_length(Hand::Hour);
        if self.show_arc && self.independent_turns.is_none() {
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
            [center, point(minute, hand_length(Hand::Minute))],
            Stroke::new(radius * 0.021, ink),
        );
        if self.show_seconds {
            painter.line_segment(
                [center, point(second, hand_length(Hand::Second))],
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
        let turns = [0.0; 3];
        for hand in Hand::ALL {
            let offset = vec2(0.0, -200.0 * hand_length(hand));
            assert_eq!(hit_hand(offset, 200.0, turns, true), Some(hand));
        }
        let turns = Hand::ALL.map(|hand| hand.turn(ClockTime::new(3, 0, 30)));
        assert_eq!(hit_hand(vec2(0.0, 185.0), 200.0, turns, false), None);
        assert_eq!(hit_hand(Vec2::ZERO, 200.0, turns, true), None);
    }

    use egui::{Event, RawInput, Rect, pos2};

    fn clock_frame(ctx: &egui::Context, app: &mut ClockApp, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 400.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.clock_face(ui));
        output.textures_delta.clear();
    }

    fn button(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn pointer_drag_moves_the_grabbed_hand_until_release() {
        let ctx = egui::Context::default();
        let mut app = ClockApp::new(ClockTime::new(3, 0, 0));
        let mut frame = |events| {
            clock_frame(&ctx, &mut app, events);
            app.time
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

    #[test]
    fn independent_drag_moves_only_the_grabbed_hand_and_preserves_linked_time() {
        let ctx = egui::Context::default();
        let linked_time = ClockTime::new(3, 0, 0);
        let mut app = ClockApp::new(linked_time);
        let original_turns = app.hand_turns();
        app.set_independent(true);
        assert_eq!(app.hand_turns(), original_turns);

        let top = pos2(200.0, 58.0);
        let bottom = pos2(200.0, 342.0);
        clock_frame(&ctx, &mut app, vec![]);
        clock_frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(top), button(top, true)],
        );
        clock_frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(bottom), button(bottom, false)],
        );
        assert_eq!(app.hand_turns(), [0.25, 0.5, 0.0]);

        let right = pos2(292.0, 200.0);
        let halfway = pos2(289.0, 224.0);
        clock_frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(right), button(right, true)],
        );
        clock_frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(halfway), button(halfway, false)],
        );
        let [hour, minute, second] = app.hand_turns();
        assert!((hour - 3.5 / 12.0).abs() < 0.001);
        assert_eq!((minute, second), (0.5, 0.0));
        assert_eq!(app.time, linked_time);

        app.set_independent(false);
        assert_eq!(app.hand_turns(), original_turns);
    }
}
