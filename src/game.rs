use piston_window::{context::Context, rectangle, types, G2d, Key};
use serde::{Deserialize, Serialize};

pub type GameInt = u16;
pub type EntityId = usize;

const MOVE_INCREMENT: GameInt = 1;
const SQUARE_1: EntityId = 0;
const SQUARE_2: EntityId = 1;
const IMMOVEABLE_OBJECT: EntityId = 2;
const BLACK: types::Rectangle<f32> = [0.0, 0.0, 0.0, 1.0];
const RED: types::Rectangle<f32> = [1.0, 0.0, 0.0, 1.0];
const GREEN: types::Rectangle<f32> = [0.0, 1.0, 0.0, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: GameInt,
    pub y: GameInt,
}

impl Point {
    pub fn new(x: GameInt, y: GameInt) -> Point {
        Point { x, y }
    }

    pub fn is_below(self, other: Point) -> bool {
        self.y > other.y
    }

    pub fn is_right_of(self, other: Point) -> bool {
        self.x > other.x
    }

    pub fn at_x(&self, x: GameInt) -> Self {
        Point { x, y: self.y }
    }

    pub fn at_y(&self, y: GameInt) -> Self {
        Point { x: self.x, y }
    }
}

impl std::ops::Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Mul<GameInt> for Point {
    type Output = Self;

    fn mul(self, multiplier: GameInt) -> Self {
        Self {
            x: self.x * multiplier,
            y: self.y * multiplier,
        }
    }
}

impl std::ops::Div<GameInt> for Point {
    type Output = Self;

    fn div(self, divisor: GameInt) -> Self {
        Self {
            x: self.x / divisor,
            y: self.y / divisor,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub bottom_right: Point,
    pub positions: Vec<Rectangle>,
    pub moveable: Vec<bool>,
    pub colors: Vec<types::Rectangle<f32>>,
}

impl Game {
    pub fn new(bottom_right: Point, square_side_length: GameInt) -> Game {
        let square1 = Rectangle::new(Point::new(0, 0), square_side_length, square_side_length);
        let square2 = Rectangle::new(
            Point::new(0, bottom_right.y - square_side_length),
            square_side_length,
            square_side_length,
        );
        let square3 = Rectangle::new(bottom_right / 2, square_side_length, square_side_length);
        Game {
            bottom_right,
            positions: vec![square1, square2, square3],
            moveable: vec![true, true, false],
            colors: vec![BLACK, RED, GREEN],
        }
    }

    pub fn entities(&self) -> &[Rectangle] {
        &self.positions
    }

    pub fn move_entity(
        &mut self,
        entity: EntityId,
        get_overlap: impl Fn(&Rectangle) -> GameInt + Copy,
        // (&mut self, diff: GameInt, width: GameInt)
        forward: impl Fn(&mut Rectangle, GameInt, GameInt) + Copy,
        // (&mut self, diff: GameInt, width: GameInt)
        backward: impl Fn(&mut Rectangle, GameInt, GameInt) + Copy,
    ) -> GameInt {
        let game_width = self.width();
        let bottom_right = self.bottom_right;
        forward(&mut self.positions[entity], MOVE_INCREMENT, game_width);
        let mut entity_segments = vec![];
        self.positions[entity].segments(bottom_right, |r| entity_segments.push(r));
        let mut overlap = 0;
        for id in 0..self.positions.len() {
            if id == entity {
                continue;
            }
            let entity_overlap = entity_segments
                .iter()
                .map(|entity_segment| {
                    let mut overlap = 0;
                    self.positions[id].segments(bottom_right, |r| {
                        match entity_segment.overlap(&r) {
                            Some(r) => overlap = overlap.max(get_overlap(&r)),
                            None => {}
                        }
                    });
                    overlap
                })
                .max();
            if let Some(entity_overlap) = entity_overlap.filter(|overlap| *overlap > 0) {
                if self.moveable[id] {
                    let pushed = self.move_entity(id, get_overlap, forward, backward);
                    overlap = overlap.max(entity_overlap - pushed);
                } else {
                    overlap = overlap.max(entity_overlap)
                }
            }
        }
        if overlap > 0 {
            backward(&mut self.positions[entity], overlap, game_width);
        }
        MOVE_INCREMENT - overlap
    }

    pub fn move_entity_up(&mut self, entity: EntityId) {
        self.move_entity(
            entity,
            |r| r.height,
            Rectangle::move_up,
            Rectangle::move_down,
        );
    }

    pub fn move_entity_left(&mut self, entity: EntityId) {
        self.move_entity(
            entity,
            |r| r.width,
            Rectangle::move_left,
            Rectangle::move_right,
        );
    }

    pub fn move_entity_right(&mut self, entity: EntityId) {
        self.move_entity(
            entity,
            |r| r.width,
            Rectangle::move_right,
            Rectangle::move_left,
        );
    }

    pub fn move_entity_down(&mut self, entity: EntityId) {
        self.move_entity(
            entity,
            |r| r.height,
            Rectangle::move_down,
            Rectangle::move_up,
        );
    }

    /// Modifies the game state according to the given keypress
    ///
    /// Returns: true if the game state was modified, false otherwise.
    pub fn process_key(&mut self, key: &Key) -> bool {
        match key {
            &Key::W => {
                self.move_entity_up(SQUARE_1);
                true
            }
            &Key::A => {
                self.move_entity_left(SQUARE_1);
                true
            }
            &Key::S => {
                self.move_entity_down(SQUARE_1);
                true
            }
            &Key::D => {
                self.move_entity_right(SQUARE_1);
                true
            }
            &Key::Up => {
                self.move_entity_up(SQUARE_2);
                true
            }
            &Key::Left => {
                self.move_entity_left(SQUARE_2);
                true
            }
            &Key::Down => {
                self.move_entity_down(SQUARE_2);
                true
            }
            &Key::Right => {
                self.move_entity_right(SQUARE_2);
                true
            }
            _ => false,
        }
    }

    pub fn tick(&mut self) {
        self.move_entity_up(IMMOVEABLE_OBJECT);
    }

    pub fn draw(&mut self, c: Context, g: &mut G2d) {
        let [x, y] = c.viewport.unwrap().window_size;
        for (i, entity) in self.entities().iter().enumerate() {
            entity.segments(self.bottom_right, |rect| {
                rectangle(
                    self.colors[i],
                    rect.scale(
                        x / self.bottom_right.x as f64,
                        y / self.bottom_right.y as f64,
                    ),
                    c.transform,
                    g,
                );
            });
        }
    }

    pub fn width(&self) -> GameInt {
        self.bottom_right.x
    }

    pub fn height(&self) -> GameInt {
        self.bottom_right.y
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    pub top_left: Point,
    pub width: GameInt,
    pub height: GameInt,
}

impl Rectangle {
    pub fn new(top_left: Point, width: GameInt, height: GameInt) -> Self {
        Rectangle {
            top_left,
            width,
            height,
        }
    }

    pub fn move_left(&mut self, diff: GameInt, width: GameInt) {
        self.top_left.x = (width + self.top_left.x - (diff % width)) % width;
    }

    pub fn move_right(&mut self, diff: GameInt, width: GameInt) {
        self.top_left.x = (self.top_left.x + diff) % width;
    }

    pub fn move_up(&mut self, diff: GameInt, height: GameInt) {
        self.top_left.y = (height + self.top_left.y - (diff % height)) % height;
    }

    pub fn move_down(&mut self, diff: GameInt, height: GameInt) {
        self.top_left.y = (self.top_left.y + diff) % height;
    }

    pub fn overlap(&self, other: &Rectangle) -> Option<Rectangle> {
        let self_bottom_right = self.bottom_right();
        let other_bottom_right = other.bottom_right();
        if self.top_left.x < other_bottom_right.x
            && self.top_left.y < other_bottom_right.y
            && other.top_left.x < self_bottom_right.x
            && other.top_left.y < self_bottom_right.y
        {
            let x = self.top_left.x.max(other.top_left.x);
            let y = self.top_left.y.max(other.top_left.y);
            Some(Rectangle {
                top_left: Point { x, y },
                width: self_bottom_right.x.min(other_bottom_right.x) - x,
                height: self_bottom_right.y.min(other_bottom_right.y) - y,
            })
        } else {
            None
        }
    }

    pub fn segments(&self, bottom_right: Point, mut f: impl FnMut(Rectangle)) {
        self.segments_helper(bottom_right, &mut f);
    }

    fn segments_helper(&self, bottom_right: Point, f: &mut impl FnMut(Rectangle)) {
        let my_bottom_right = self.bottom_right();
        if my_bottom_right.is_below(bottom_right) {
            let bottom_overflow = my_bottom_right.y - bottom_right.y;
            Rectangle {
                top_left: Point {
                    x: self.top_left.x,
                    y: 0,
                },
                width: self.width,
                height: bottom_overflow,
            }
            .segments_helper(bottom_right, f);
        }
        if my_bottom_right.is_right_of(bottom_right) {
            let right_overflow = my_bottom_right.x - bottom_right.x;
            Rectangle {
                top_left: Point {
                    x: 0,
                    y: self.top_left.y,
                },
                width: right_overflow,
                height: self.height,
            }
            .segments_helper(bottom_right, f);
        }
        f(Rectangle {
            top_left: self.top_left,
            width: self.width.min(bottom_right.x - self.top_left.x),
            height: self.height.min(bottom_right.y - self.top_left.y),
        });
    }

    pub fn bottom_right(&self) -> Point {
        self.top_left
            + Point {
                x: self.width,
                y: self.height,
            }
    }

    pub fn scale(&self, x_factor: f64, y_factor: f64) -> types::Rectangle<f64> {
        [
            self.top_left.x as f64 * x_factor,
            self.top_left.y as f64 * y_factor,
            self.width as f64 * x_factor,
            self.height as f64 * y_factor,
        ]
    }
}

#[test]
fn my_rectangle_segments_no_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![Rectangle::new(Point::new(5, 5), 5, 5)];
    rect.segments(Point { x: 10, y: 10 }, |r| {
        for (i, rec) in expected_recs.iter().enumerate() {
            if rec == &r {
                expected_recs.remove(i);
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}

#[test]
fn my_rectangle_segments_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![
        Rectangle::new(Point::new(5, 5), 2, 5),
        Rectangle::new(Point::new(0, 5), 3, 5),
    ];
    rect.segments(Point { x: 7, y: 10 }, |r| {
        for (i, rec) in expected_recs.iter().enumerate() {
            if rec == &r {
                expected_recs.remove(i);
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}
