//! Shared prompt driver: render a frame, read an event, repeat.
//!
//! Centralizes the event loop so each prompt only supplies a render closure
//! and an event handler. Drawing goes through the shared in-place engine, so
//! prompts behave correctly on terminals and become no-ops off-terminal.

use crate::core::render::Rendered;
use crate::error::Result;
use crate::input::Outcome;
use crate::input::event::{EventSource, InputEvent};
use crate::output::live::InPlace;

/// What a prompt's event handler decides after each event.
pub(crate) enum Flow<T> {
    /// Keep the prompt open and redraw.
    Continue,
    /// Finish with a submitted value.
    Submit(T),
    /// Finish as cancelled.
    Cancel,
    /// Finish because a registered shortcut fired (carries its id).
    Shortcut(i32),
    /// Keep open but redraw from scratch (after an external program ran).
    Refresh,
}

/// Runs a prompt loop over `source`, driving `render` and `handle`.
///
/// `render` builds the current frame from the state; its `bool` argument is
/// `true` only for the final frame drawn after submission, so widgets can hide
/// the cursor and other active-only adornments. `handle` consumes one event
/// and returns the next [`Flow`]. The final frame is left on screen.
pub(crate) fn run_prompt<S, E, R, H, T>(
    source: &mut E,
    state: &mut S,
    mut render: R,
    mut handle: H,
) -> Result<Outcome<T>>
where
    E: EventSource,
    R: FnMut(&S, bool) -> Rendered,
    H: FnMut(&mut S, InputEvent) -> Flow<T>,
{
    let mut inplace = InPlace::new(false);
    loop {
        let frame = render(state, false);
        inplace.draw(&frame)?;
        let event = source.next_event()?;
        match handle(state, event) {
            Flow::Continue => {}
            Flow::Refresh => inplace.reset(),
            Flow::Submit(value) => {
                let frame = render(state, true);
                inplace.draw(&frame)?;
                inplace.finish()?;
                return Ok(Outcome::Submitted(value));
            }
            Flow::Cancel => {
                inplace.finish()?;
                return Ok(Outcome::Cancelled);
            }
            Flow::Shortcut(id) => {
                inplace.finish()?;
                return Ok(Outcome::Shortcut(id));
            }
        }
    }
}
