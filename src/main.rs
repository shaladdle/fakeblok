extern crate log;
extern crate piston_window;
extern crate pretty_env_logger;

use log::info;
use piston_window::{
    clear, context::Context, rectangle, types::Rectangle, Button, ButtonArgs, ButtonState, Event,
    EventLoop, G2d, Input, Key, OpenGL, PistonWindow, WindowSettings,
};

const BLACK: Rectangle<f32> = [0.0, 0.0, 0.0, 1.0];

type GameInt = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    x: GameInt,
    y: GameInt,
}

impl Point {
    fn new(x: GameInt, y: GameInt) -> Point {
        Point { x, y }
    }

    fn is_below(self, other: Point) -> bool {
        return self.y > other.y;
    }

    fn is_right_of(self, other: Point) -> bool {
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

struct Game {
    bottom_right: Point,
    square1: MyRectangle,
    square2: MyRectangle,
}

impl Game {
    fn new(bottom_right: Point, square_side_length: GameInt) -> Game {
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

    fn entities(&self) -> Vec<&MyRectangle> {
        vec![&self.square1, &self.square2]
    }

    fn process_key(&mut self, key: &Key) {
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

    fn draw(&mut self, c: Context, g: &mut G2d) {
        for entity in self.entities() {
            entity.draw(self.bottom_right, &mut |rect| {
                rectangle(BLACK, rect, c.transform, g);
            });
        }
    }

    fn width(&self) -> GameInt {
        self.bottom_right.x
    }

    fn height(&self) -> GameInt {
        self.bottom_right.y
    }
}

struct MyRectangle {
    top_left: Point,
    width: GameInt,
    height: GameInt,
}

impl MyRectangle {
    fn new(top_left: Point, width: GameInt, height: GameInt) -> Self {
        MyRectangle {
            top_left,
            width,
            height,
        }
    }

    fn move_left(&mut self, diff: GameInt, width: GameInt) {
        self.top_left.x = (width + self.top_left.x - (diff % width)) % width;
    }

    fn move_right(&mut self, diff: GameInt, width: GameInt) {
        self.top_left.x = (self.top_left.x + diff) % width;
    }

    fn move_up(&mut self, diff: GameInt, height: GameInt) {
        self.top_left.y = (height + self.top_left.y - (diff % height)) % height;
    }

    fn move_down(&mut self, diff: GameInt, height: GameInt) {
        self.top_left.y = (self.top_left.y + diff) % height;
    }

    fn draw(&self, bottom_right: Point, drawer: &mut impl FnMut(Rectangle)) {
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

    fn bottom_right(&self) -> Point {
        self.top_left
            + Point {
                x: self.width,
                y: self.height,
            }
    }
}

fn get_key(event: &Event) -> Option<&Key> {
    match event {
        Event::Input(
            Input::Button(ButtonArgs {
                button: Button::Keyboard(ref key),
                state: ButtonState::Press,
                ..
            }),
            _,
        ) => Some(key),
        _ => None,
    }
}

fn main() {
    pretty_env_logger::init();
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("shapes", [512; 2])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("start!");
    let mut game = Game::new(Point { x: 200, y: 200 }, 50);
    while let Some(e) = window.next() {
        if let Some(key) = get_key(&e) {
            game.process_key(key);
        }
        window.draw_2d(&e, |c, g, _| {
            clear([1.0; 4], g);
            game.draw(c, g);
        });
    }
    info!("end :(");
}

#[test]
fn my_rectangle_draw_no_overflow() {
    let rect = MyRectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![[5., 5., 5., 5.]];
    rect.draw(Point { x: 10, y: 10 }, &mut |r| {
        for (i, rec) in expected_recs.iter().enumerate() {
            if rec == &r {
                expected_recs.remove(i);
                return
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}

#[test]
fn my_rectangle_draw_overflow() {
    let rect = MyRectangle {
        top_left: Point { x: 5, y: 5 },
        width: 5,
        height: 5,
    };
    let mut expected_recs = vec![[5., 5., 2., 5.], [0., 5., 3., 5.]];
    rect.draw(Point { x: 7, y: 10 }, &mut |r| {
        for (i, rec) in expected_recs.iter().enumerate() {
            if rec == &r {
                expected_recs.remove(i);
                return
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}
