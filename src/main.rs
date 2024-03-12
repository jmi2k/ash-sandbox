#![feature(c_str_literals)]
#![feature(stmt_expr_attributes)]
#![feature(variant_count)]

use core::time::Duration;
use std::time::Instant;

use event::{Action, EventHandler, Input};
use graphics::{render::Renderer, Graphics};
use window::Window;

mod event;
mod graphics;
mod utils;
mod window;

const TICK_DURATION: Duration = Duration::from_micros(1_000_000 / 32);
const TOO_MUCH_TIME: Duration = Duration::from_micros(8_000_000 / 32);

#[rustfmt::skip]
const BINDINGS: [(Input, Action); 1] = [
    (Input::Close, Action::Exit),
];

fn main() {
    #[rustfmt::skip]
    let window = Window::new(c"ash-sandbox", 1_024, 512)
        .expect("Failed to create window");

    let event_handler = EventHandler::from_iter(BINDINGS);
    let mut gfx = Graphics::new(&window);
    let renderer = Renderer::new(&gfx);
    let mut tick = false;

    let mut then = Instant::now();
    let mut accrued_time = Duration::ZERO;

    window.run(|event| {
        match event_handler.handle(event) {
            Action::Exit => return,
            Action::Idle => tick = true,
            _ => {}
        }

        if !tick {
            return;
        }

        tick = false;

        let now = Instant::now();
        accrued_time += now - then;
        then = now;

        while accrued_time >= TOO_MUCH_TIME {
            accrued_time -= TICK_DURATION;
        }

        while accrued_time >= TICK_DURATION {
            accrued_time -= TICK_DURATION;
        }

        gfx.prepare_frame(|frame| renderer.render(frame));
    });
}
