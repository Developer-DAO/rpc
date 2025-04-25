pub struct OnlyOnce {
    _redirected_once: bool,
}

#[derive(Debug)]
pub struct Policy {
    redirects_remaining: u8,
    pub state: State,
}

#[derive(Debug)]
pub enum State {
    Continue,
    Return,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            redirects_remaining: 1,
            state: State::Continue,
        }
    }
}

impl Policy {
    pub fn apply_redirect_policy(&mut self) -> State {
        if self.redirects_remaining == 0 {
            State::Return
        } else {
            self.redirects_remaining -= 1;
            State::Continue
        }
    }
}
