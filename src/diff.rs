/// Compare two Dota 2 game states to produce game events
///
/// This provides a [`Diffable`] trait implemented for game [`crate::components`].
use crate::event::GameEvent;

/// Diffable trait to compare a game component with a newer one
pub trait Diffable {
    fn diff<'a>(&'a self, new: &'a Self) -> Vec<GameEvent>;
}
