use log::info;

use piston_window::{context::Context, rectangle, types::Rectangle, G2d, Key};

pub type GameInt = u16;

const BLACK: Rectangle<f32> = [0.0, 0.0, 0.0, 1.0];

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
    pub square1: MyRectangle,
    pub square2: MyRectangle,
}

impl Game {
    pub fn new(bottom_right: Point, square_side_length: GameInt) -> Game {
        let square1 = MyRectangle::new(Point::new(0, 0), square_side_length, square_side_length);
        let square2_y = bottom_right.y - square_side_length;
        let square2 = MyRectangle::new(
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

    pub fn entities(&self) -> Vec<&MyRectangle> {
        vec![&self.square1, &self.square2]
    }

    pub fn process_key(&mut self, key: &Key) {
        match key {
            &Key::W => self.square1.move_up(5, self.height()),
            &Key::A => self.square1.move_left(5, self.width()),
            &Key::S => self.square1.move_down(5, self.height()),
            &Key::D => self.square1.move_right(5, self.width()),
            &Key::Up => self.square2.move_up(5, self.height()),
            &Key::Left => self.square2.move_left(5, self.width()),
            &Key::Down => self.square2.move_down(5, self.height()),
            &Key::Right => self.square2.move_right(5, self.width()),
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

pub struct MyRectangle {
    pub top_left: Point,
    pub width: GameInt,
    pub height: GameInt,
}

impl MyRectangle {
    pub fn new(top_left: Point, width: GameInt, height: GameInt) -> Self {
        MyRectangle {
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

    pub fn draw(&self, bottom_right: Point, drawer: &mut impl FnMut(Rectangle)) {
        let my_bottom_right = self.bottom_right();
        if my_bottom_right.is_below(bottom_right) {
            let bottom_overflow = my_bottom_right.y - bottom_right.y;
            MyRectangle {
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
            MyRectangle {
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
