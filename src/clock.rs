#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl ClockTime {
    pub fn new(hour: u32, minute: u32, second: u32) -> Self {
        assert!(hour < 24 && minute < 60 && second < 60);
        Self {
            hour,
            minute,
            second,
        }
    }

    pub fn second_turn(self) -> f32 {
        self.second as f32 / 60.0
    }

    pub fn minute_turn(self) -> f32 {
        (self.minute as f32 + self.second_turn()) / 60.0
    }

    pub fn hour_start_turn(self) -> f32 {
        (self.hour % 12) as f32 / 12.0
    }

    pub fn hour_turn(self) -> f32 {
        self.hour_start_turn() + self.minute_turn() / 12.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Hour,
    Minute,
    Second,
}

impl Hand {
    pub fn turn(self, time: ClockTime) -> f32 {
        match self {
            Self::Hour => time.hour_turn(),
            Self::Minute => time.minute_turn(),
            Self::Second => time.second_turn(),
        }
    }
}

pub fn pointer_turn(dx: f32, dy: f32) -> f32 {
    dx.atan2(-dy) / std::f32::consts::TAU
}

pub struct HandDrag {
    hand: Hand,
    start: ClockTime,
    previous_turn: f32,
    turns: f32,
}

impl HandDrag {
    pub fn new(hand: Hand, start: ClockTime, pointer_turn: f32) -> Self {
        Self {
            hand,
            start,
            previous_turn: pointer_turn,
            turns: 0.0,
        }
    }

    pub fn move_to(&mut self, turn: f32) -> ClockTime {
        self.turns += (turn - self.previous_turn + 0.5).rem_euclid(1.0) - 0.5;
        self.previous_turn = turn;
        let seconds = match self.hand {
            Hand::Hour => (self.turns * 720.0).round() as i32 * 60,
            Hand::Minute => (self.turns * 60.0).round() as i32 * 60,
            Hand::Second => (self.turns * 60.0).round() as i32,
        };
        let start = self.start.hour * 3600 + self.start.minute * 60 + self.start.second;
        let total = (start as i32 + seconds).rem_euclid(86_400) as u32;
        ClockTime::new(total / 3600, total / 60 % 60, total % 60)
    }
}

pub fn direction(turn: f32) -> (f32, f32) {
    let (sin, cos) = (turn * std::f32::consts::TAU).sin_cos();
    (sin, -cos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
    }

    #[test]
    fn midnight_and_noon_have_no_elapsed_arc() {
        for hour in [0, 12] {
            let t = ClockTime::new(hour, 0, 0);
            close(t.hour_start_turn(), 0.0);
            close(t.hour_turn(), 0.0);
            close(t.minute_turn(), 0.0);
        }
    }

    #[test]
    fn half_past_points_between_hours() {
        let t = ClockTime::new(15, 30, 0);
        close(t.hour_start_turn(), 3.0 / 12.0);
        close(t.hour_turn(), 3.5 / 12.0);
        close(t.minute_turn(), 0.5);
    }

    #[test]
    fn last_second_before_midnight() {
        let t = ClockTime::new(23, 59, 59);
        close(t.second_turn(), 59.0 / 60.0);
        close(t.minute_turn(), 3599.0 / 3600.0);
        close(t.hour_turn(), 43199.0 / 43200.0);
    }

    #[test]
    fn minute_drag_crosses_midnight_in_both_directions() {
        let start = ClockTime::new(23, 59, 15);
        let mut drag = HandDrag::new(Hand::Minute, start, 59.0 / 60.0);
        assert_eq!(drag.move_to(0.0), ClockTime::new(0, 0, 15));
        assert_eq!(drag.move_to(59.0 / 60.0), start);
    }

    #[test]
    fn hour_drag_advances_minutes_and_accumulates_a_full_turn() {
        let mut drag = HandDrag::new(Hand::Hour, ClockTime::new(3, 0, 0), 0.25);
        assert_eq!(drag.move_to(3.5 / 12.0), ClockTime::new(3, 30, 0));
        for turn in [0.5, 0.75, 0.0] {
            drag.move_to(turn);
        }
        assert_eq!(drag.move_to(0.25), ClockTime::new(15, 0, 0));
    }

    #[test]
    fn second_drag_accumulates_small_movements() {
        let start = ClockTime::new(10, 59, 59);
        let mut drag = HandDrag::new(Hand::Second, start, -1.0 / 60.0);
        assert_eq!(drag.move_to(-0.7 / 60.0), start);
        assert_eq!(drag.move_to(-0.4 / 60.0), ClockTime::new(11, 0, 0));
    }

    #[test]
    fn dial_runs_clockwise_from_the_top() {
        for (turn, x, y) in [
            (0.0, 0.0, -1.0),
            (0.25, 1.0, 0.0),
            (0.5, 0.0, 1.0),
            (0.75, -1.0, 0.0),
        ] {
            let (dx, dy) = direction(turn);
            close(dx, x);
            close(dy, y);
            close(pointer_turn(x, y).rem_euclid(1.0), turn);
        }
    }
}
