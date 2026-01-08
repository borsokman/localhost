use super::{Event, Poller};

pub struct EventLoop {
    poller: Poller,
}

impl EventLoop {
    pub fn new() -> Result<Self, String> {
        Ok(Self { poller: Poller::new()? })
    }

    pub fn tick<F>(&self, max_events: usize, timeout_ms: Option<i32>, mut handler: F) -> Result<(), String>
    where
        F: FnMut(&Event),
    {
        let events = self.poller.wait(max_events, timeout_ms)?;
        for ev in events.iter() {
            handler(ev);
        }
        Ok(())
    }

    pub fn poller(&self) -> &Poller {
        &self.poller
    }
}