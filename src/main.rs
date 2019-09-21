extern crate log;
extern crate piston_window;
extern crate pretty_env_logger;

use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, Input, Key, OpenGL, PistonWindow,
    WindowSettings,
};

mod game;

use game::*;

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
                return;
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
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}
