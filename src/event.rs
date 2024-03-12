pub enum Event {
    Idle,
    Input(Input),
}

#[derive(Eq, PartialEq)]
pub enum Input {
    Close,
}

#[derive(Copy, Clone, Default)]
pub enum Action {
    #[default]
    Nop,

    Idle,
    Exit,

    #[allow(unused)]
    Debug(&'static str),
}

#[derive(Default)]
pub struct EventHandler {
    on_close: Action,
}

impl EventHandler {
    pub fn handle(&self, event: Event) -> Action {
        match event {
            Event::Idle => Action::Idle,
            Event::Input(input) => self.handle_input(input),
        }
    }

    fn handle_input(&self, input: Input) -> Action {
        match input {
            Input::Close => self.on_close,
        }
    }
}

impl FromIterator<(Input, Action)> for EventHandler {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (Input, Action)>,
    {
        let mut ego = Self::default();

        for (input, action) in iter {
            match input {
                Input::Close => ego.on_close = action,
            }
        }

        ego
    }
}
