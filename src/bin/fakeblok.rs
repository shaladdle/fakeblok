use fakeblok::game::{Game, Point};
use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Loop,
    OpenGL, PistonWindow, WindowSettings,
};
use pretty_env_logger;

fn process_input(game: &mut Game, input: &Input) {
    if let Input::Button(ButtonArgs {
        button: Button::Keyboard(key),
        state: ButtonState::Press,
        ..
    }) = input
    {
        game.process_key(key);
    }
}

fn process_loop(game: &mut Game, lp: &Loop) {
    match lp {
        Loop::Idle(_) => {}
        Loop::Update(_) => {
            game.move_entity_up(2);
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

    let mut events = Events::new(EventSettings::new().ups(5));
    info!("start!");
    let mut game = Game::new(Point { x: 200, y: 200 }, 50);
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => process_input(&mut game, input),
            Event::Loop(Loop::Render(_)) => {
                window.draw_2d(&event, |c, g, _| {
                    clear([1.0; 4], g);
                    game.draw(c, g);
                });
            }
            Event::Loop(ref lp) => process_loop(&mut game, lp),
            _ => {}
        }
    }
    info!("end :(");
}
