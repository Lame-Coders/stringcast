mod engine;

pub use engine::{
    detect_trigger, finalize_pending_dynamic, DetectionDecision, PendingDynamicTrigger,
    TriggerMatch, DYNAMIC_DEBOUNCE_MS,
};
