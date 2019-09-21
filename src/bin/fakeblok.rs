use fakeblok::game::*;
use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, Input, Key, OpenGL, PistonWindow,
    WindowSettings,
};
use pretty_env_logger;

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
