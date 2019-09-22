use fakeblok::game::{Game, Point};
use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Key,
    Loop, OpenGL, PistonWindow, WindowSettings,
};
use pretty_env_logger;
use std::collections::HashSet;

fn process_input(keys: &mut HashSet<Key>, input: &Input) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) => match state {
            ButtonState::Press => {
                keys.insert(*key);
            }
            ButtonState::Release => {
                keys.remove(key);
            }
        },
        _ => {}
    }
}

fn process_loop(game: &mut Game, lp: &Loop, keys: &HashSet<Key>) {
    match lp {
        Loop::Idle(_) => {}
        Loop::Update(_) => {
            game.move_entity_up(2);
            for key in keys {
                game.process_key(key);
            }
        }
        Loop::AfterRender(_) => {}
        lp => panic!("Didn't expect {:?}", lp),
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

    let mut events = Events::new(EventSettings::new().ups(1000));
    info!("start!");
    let mut game = Game::new(
        Point {
            x: 10_000,
            y: 10_000,
        },
        1000,
    );
    let mut keys = HashSet::new();
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => process_input(&mut keys, input),
            Event::Loop(Loop::Render(_)) => {
                window.draw_2d(&event, |c, g, _| {
                    clear([1.0; 4], g);
                    game.draw(c, g);
                });
            }
            Event::Loop(ref lp) => process_loop(&mut game, lp, &keys),
            _ => {}
        }
    }
    info!("end :(");
}
