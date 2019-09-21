use log::info;

use piston_window::{context::Context, rectangle, types, G2d, Key};
use std::convert::{TryFrom, TryInto};

pub type GameInt = u16;

const BLACK: types::Rectangle<f32> = [0.0, 0.0, 0.0, 1.0];

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
        return self.y > other.y;
    }

    pub fn is_right_of(self, other: Point) -> bool {
        return self.x > other.x;
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

pub struct Game {
    pub bottom_right: Point,
    pub square1: Rectangle,
    pub square2: Rectangle,
}

impl Game {
    pub fn new(bottom_right: Point, square_side_length: GameInt) -> Game {
        let square1 = Rectangle::new(Point::new(0, 0), square_side_length, square_side_length);
        let square2_y = bottom_right.y - square_side_length;
        let square2 = Rectangle::new(
            Point::new(0, square2_y),
            square_side_length,
            square_side_length,
        );
        Game {
            bottom_right,
            square1,
            square2,
        }
    }

    pub fn entities(&self) -> Vec<&Rectangle> {
        vec![&self.square1, &self.square2]
    }

    pub fn process_key(&mut self, key: &Key) {
        match key {
            &Key::W => self.square1.move_up(5, self.height(), &mut self.square2),
            &Key::A => self.square1.move_left(5, self.width(), &mut self.square2),
            &Key::S => self.square1.move_down(5, self.height(), &mut self.square2),
            &Key::D => self.square1.move_right(5, self.width(), &mut self.square2),
            &Key::Up => self.square2.move_up(5, self.height(), &mut self.square1),
            &Key::Left => self.square2.move_left(5, self.width(), &mut self.square1),
            &Key::Down => self.square2.move_down(5, self.height(), &mut self.square1),
            &Key::Right => self.square2.move_right(5, self.width(), &mut self.square1),
            _ => (),
        }
        info!("key: {:?}", key);
    }

    pub fn draw(&mut self, c: Context, g: &mut G2d) {
        for entity in self.entities() {
            entity.draw(self.bottom_right, &mut |rect| {
                rectangle(BLACK, rect, c.transform, g);
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

#[derive(Clone, Copy, Debug)]
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

    pub fn move_left(&mut self, diff: GameInt, width: GameInt, other: &mut Rectangle) {
        self.top_left.x = (width + self.top_left.x - (diff % width)) % width;
        match self.overlap(other) {
            Some(Rectangle {
                top_left, width, ..
            }) => self.top_left.x = top_left.x + width,
            None => {}
        }
    }

    pub fn move_right(&mut self, diff: GameInt, width: GameInt, other: &mut Rectangle) {
        self.top_left.x = (self.top_left.x + diff) % width;
        let right_edge = self.top_left.x + self.width;
        if right_edge > width {
            let overflow = Rectangle {
                top_left: self.top_left.at_x(0),
                width: right_edge % width,
                height: self.height,
            };
            match overflow.overlap(other) {
                Some(Rectangle { width, .. }) => self.top_left.x -= width,
                None => {}
            }
        }
        match self.overlap(other) {
            Some(Rectangle { width, .. }) => self.top_left.x -= width,
            None => {}
        }
    }

    pub fn move_up(&mut self, diff: GameInt, height: GameInt, other: &mut Rectangle) {
        self.top_left.y = (height + self.top_left.y - (diff % height)) % height;
        match self.overlap(other) {
            Some(Rectangle {
                top_left, height, ..
            }) => self.top_left.y = top_left.y + height,
            None => {}
        }
    }

    pub fn move_down(&mut self, diff: GameInt, height: GameInt, other: &mut Rectangle) {
        self.top_left.y = (self.top_left.y + diff) % height;
        let bottom_edge = self.top_left.y + self.height;
        if bottom_edge > height {
            let overflow = Rectangle {
                top_left: self.top_left.at_y(0),
                width: self.width,
                height: bottom_edge % height,
            };
            match overflow.overlap(other) {
                Some(Rectangle { height, .. }) => self.top_left.y -= height,
                None => {}
            }
        }
        match self.overlap(other) {
            Some(Rectangle { height, .. }) => self.top_left.y -= height,
            None => {}
        }
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

    pub fn draw(&self, bottom_right: Point, drawer: &mut impl FnMut(types::Rectangle)) {
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
            .draw(bottom_right, drawer);
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
            .draw(bottom_right, drawer);
        }
        drawer([
            f64::from(self.top_left.x),
            self.top_left.y.into(),
            self.width.min(bottom_right.x - self.top_left.x).into(),
            self.height.min(bottom_right.y - self.top_left.y).into(),
        ]);
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
fn my_rectangle_draw_no_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![[5., 5., 5., 5.]];
    rect.draw(Point { x: 10, y: 10 }, &mut |r| {
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
fn my_rectangle_draw_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![[5., 5., 2., 5.], [0., 5., 3., 5.]];
    rect.draw(Point { x: 7, y: 10 }, &mut |r| {
        for (i, rec) in expected_recs.iter().enumerate() {
            if rec == &r {
                expected_recs.remove(i);
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}
