use singalong_core::model::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsAdapter as NativeAdapter;
#[derive(Clone)]
pub struct Cancellation {
    pub epoch: Arc<AtomicU64>,
    pub expected: u64,
}
impl Cancellation {
    pub fn cancelled(&self) -> bool {
        self.epoch.load(Ordering::SeqCst) != self.expected
    }
}
pub struct Observation {
    pub players: Vec<PlayerInfo>,
    pub sessions: Vec<PlaybackSnapshot>,
}
pub trait Adapter {
    fn changed(&self) -> bool {
        false
    }
    fn refresh(
        &mut self,
        enabled: &[PlayerId],
        now: u64,
        cancel: &Cancellation,
    ) -> Result<Observation, &'static str>;
}
#[cfg(not(windows))]
pub struct NativeAdapter;
#[cfg(not(windows))]
impl NativeAdapter {
    pub fn new(_: &Cancellation) -> Result<Self, &'static str> {
        Err("Playback observation is currently available on Windows only.")
    }
}
#[cfg(not(windows))]
impl Adapter for NativeAdapter {
    fn refresh(
        &mut self,
        _: &[PlayerId],
        _: u64,
        _: &Cancellation,
    ) -> Result<Observation, &'static str> {
        Err("Playback observation unavailable.")
    }
}
