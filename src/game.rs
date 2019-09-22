use log::info;

use piston_window::{context::Context, rectangle, types, G2d, Key};
use std::convert::{TryFrom, TryInto};

pub type GameInt = u16;
pub type EntityId = usize;

const MOVE_INCREMENT: GameInt = 5;
const SQUARE_1: EntityId = 0;
const SQUARE_2: EntityId = 1;
const BLACK: types::Rectangle<f32> = [0.0, 0.0, 0.0, 1.0];
const RED: types::Rectangle<f32> = [1.0, 0.0, 0.0, 1.0];
const GREEN: types::Rectangle<f32> = [0.0, 1.0, 0.0, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
            if id == entity { continue }
            let entity_overlap = entity_segments.iter().map(|entity_segment| {
                let mut overlap = 0;
                self.positions[id].segments(bottom_right, |r| match entity_segment.overlap(&r) {
                    Some(r) => overlap = overlap.max(get_overlap(&r)),
                    None => {}
                });
                overlap
            }).max();
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
        self.move_entity(entity, |r| r.height, Rectangle::move_up, Rectangle::move_down);
    }

    pub fn move_entity_left(&mut self, entity: EntityId) {
        self.move_entity(entity, |r| r.width, Rectangle::move_left, Rectangle::move_right);
    }

    pub fn move_entity_right(&mut self, entity: EntityId) {
        self.move_entity(entity, |r| r.width, Rectangle::move_right, Rectangle::move_left);
    }

    pub fn move_entity_down(&mut self, entity: EntityId) {
        self.move_entity(entity, |r| r.height, Rectangle::move_down, Rectangle::move_up);
    }

    pub fn process_key(&mut self, key: &Key) {
        match key {
            &Key::W => self.move_entity_up(SQUARE_1),
            &Key::A => self.move_entity_left(SQUARE_1),
            &Key::S => self.move_entity_down(SQUARE_1),
            &Key::D => self.move_entity_right(SQUARE_1),
            &Key::Up => self.move_entity_up(SQUARE_2),
            &Key::Left => self.move_entity_left(SQUARE_2),
            &Key::Down => self.move_entity_down(SQUARE_2),
            &Key::Right => self.move_entity_right(SQUARE_2),
            _ => (),
        }
        info!("key: {:?}", key);
    }

    pub fn draw(&mut self, c: Context, g: &mut G2d) {
        for (i, entity) in self.entities().iter().enumerate() {
            entity.segments(self.bottom_right, |rect| {
                rectangle(
                    self.colors[i],
                    Into::<types::Rectangle>::into(rect),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rectangle {
    pub top_left: Point,
    pub width: GameInt,
    pub height: GameInt,
}

impl<T> Into<types::Rectangle<T>> for Rectangle
where
    GameInt: Into<T>,
{
    fn into(self) -> types::Rectangle<T> {
        [
            self.top_left.x.into(),
            self.top_left.y.into(),
            self.width.into(),
            self.height.into(),
        ]
    }
}

impl<T: Copy> TryFrom<types::Rectangle<T>> for Rectangle
where
    GameInt: TryFrom<T>,
{
    type Error = <GameInt as TryFrom<T>>::Error;
    fn try_from([x, y, w, h]: types::Rectangle<T>) -> Result<Self, <GameInt as TryFrom<T>>::Error> {
        Ok(Rectangle {
            top_left: Point {
                x: x.try_into()?,
                y: y.try_into()?,
            },
            width: w.try_into()?,
            height: h.try_into()?,
        })
    }
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
