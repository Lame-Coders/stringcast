mod buffer;
mod controller;
mod events;
mod guard;
mod hook;
mod rdev_hook;
mod simulator;

pub use buffer::{BufferEvent, KeystrokeBuffer};
pub use controller::{InputController, InputControllerError, InputControllerOutcome};
pub use events::{InputEvent, KeyShortcut, NavigationKey};
pub use guard::{SyntheticInputGuard, SyntheticInputGuardToken};
pub use hook::{InputHook, InputHookError};
pub use rdev_hook::{RdevEventNormalizer, RdevInputHook};
pub use simulator::{
    EnigoInputSimulator, GuardedInputSimulator, InputSimulationError, InputSimulator,
    RecordedInputAction, RecordingInputSimulator,
};
