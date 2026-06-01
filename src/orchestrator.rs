use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Idle,
    PendingDynamicTrigger,
    Extracting,
    CallingApi,
    Replacing,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    FocusChanged,
    AppExcluded,
    UserPaused,
    EscapePressed,
    SleepOrLock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationSnapshot {
    pub operation_id: u64,
    pub app_id: String,
    pub window_id: Option<String>,
    pub extracted_text: String,
    pub replacement_target_text: String,
    pub transform_input: String,
    pub trigger_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedOperation {
    pub app_id: String,
    pub window_id: Option<String>,
    pub trigger_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    AlreadyActive,
    InvalidTransition {
        from: OperationState,
        to: OperationState,
    },
}

#[derive(Debug, Clone)]
pub struct OperationOrchestrator {
    state: OperationState,
    active_operation_id: Option<u64>,
    active_snapshot: Option<OperationSnapshot>,
    queued: VecDeque<QueuedOperation>,
    next_operation_id: u64,
}

impl Default for OperationOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationOrchestrator {
    pub fn new() -> Self {
        Self {
            state: OperationState::Idle,
            active_operation_id: None,
            active_snapshot: None,
            queued: VecDeque::new(),
            next_operation_id: 1,
        }
    }

    pub fn state(&self) -> OperationState {
        self.state
    }

    pub fn active_snapshot(&self) -> Option<&OperationSnapshot> {
        self.active_snapshot.as_ref()
    }

    pub fn queued(&self) -> Option<&QueuedOperation> {
        self.queued.back()
    }

    pub fn begin_static_extraction(&mut self) -> Result<u64, OrchestratorError> {
        self.begin_operation(OperationState::Extracting)
    }

    pub fn begin_pending_dynamic(&mut self) -> Result<u64, OrchestratorError> {
        self.begin_operation(OperationState::PendingDynamicTrigger)
    }

    pub fn complete_extraction(
        &mut self,
        snapshot: OperationSnapshot,
    ) -> Result<(), OrchestratorError> {
        self.ensure_transition(OperationState::CallingApi)?;
        self.active_snapshot = Some(snapshot);
        self.state = OperationState::CallingApi;
        Ok(())
    }

    pub fn begin_replacement(&mut self) -> Result<(), OrchestratorError> {
        self.transition(OperationState::Replacing)
    }

    pub fn begin_verification(&mut self) -> Result<(), OrchestratorError> {
        self.transition(OperationState::Verifying)
    }

    pub fn complete(&mut self) -> Result<Option<QueuedOperation>, OrchestratorError> {
        self.ensure_transition(OperationState::Completed)?;
        self.state = OperationState::Completed;
        self.clear_active();
        Ok(self.pop_queued())
    }

    pub fn fail(&mut self) -> Option<QueuedOperation> {
        self.state = OperationState::Failed;
        self.clear_active();
        self.pop_queued()
    }

    pub fn cancel(&mut self, _reason: CancelReason) -> Option<QueuedOperation> {
        self.state = OperationState::Cancelled;
        self.clear_active();
        self.queued.clear();
        None
    }

    pub fn queue_latest(&mut self, operation: QueuedOperation) {
        self.queued.clear();
        self.queued.push_back(operation);
    }

    fn begin_operation(&mut self, state: OperationState) -> Result<u64, OrchestratorError> {
        if self.state != OperationState::Idle {
            return Err(OrchestratorError::AlreadyActive);
        }

        let id = self.next_operation_id;
        self.next_operation_id += 1;
        self.active_operation_id = Some(id);
        self.state = state;
        Ok(id)
    }

    fn transition(&mut self, to: OperationState) -> Result<(), OrchestratorError> {
        self.ensure_transition(to)?;
        self.state = to;
        Ok(())
    }

    fn ensure_transition(&self, to: OperationState) -> Result<(), OrchestratorError> {
        if is_allowed_transition(self.state, to) {
            Ok(())
        } else {
            Err(OrchestratorError::InvalidTransition {
                from: self.state,
                to,
            })
        }
    }

    fn clear_active(&mut self) {
        self.state = OperationState::Idle;
        self.active_operation_id = None;
        self.active_snapshot = None;
    }

    fn pop_queued(&mut self) -> Option<QueuedOperation> {
        self.queued.pop_back()
    }
}

fn is_allowed_transition(from: OperationState, to: OperationState) -> bool {
    matches!(
        (from, to),
        (
            OperationState::PendingDynamicTrigger,
            OperationState::Extracting
        ) | (OperationState::Extracting, OperationState::CallingApi)
            | (OperationState::CallingApi, OperationState::Replacing)
            | (OperationState::Replacing, OperationState::Verifying)
            | (OperationState::Verifying, OperationState::Completed)
            | (OperationState::Replacing, OperationState::Completed)
            | (OperationState::CallingApi, OperationState::Failed)
            | (OperationState::Replacing, OperationState::Failed)
            | (OperationState::Verifying, OperationState::Failed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_follows_happy_path() {
        let mut orchestrator = OperationOrchestrator::new();
        let id = orchestrator.begin_static_extraction().unwrap();

        orchestrator
            .complete_extraction(OperationSnapshot {
                operation_id: id,
                app_id: "com.example.App".to_string(),
                window_id: Some("window-1".to_string()),
                extracted_text: "hello ?fix".to_string(),
                replacement_target_text: "hello ?fix".to_string(),
                transform_input: "hello".to_string(),
                trigger_text: "?fix".to_string(),
            })
            .unwrap();
        orchestrator.begin_replacement().unwrap();
        orchestrator.begin_verification().unwrap();
        let queued = orchestrator.complete().unwrap();

        assert_eq!(queued, None);
        assert_eq!(orchestrator.state(), OperationState::Idle);
        assert!(orchestrator.active_snapshot().is_none());
    }

    #[test]
    fn only_latest_queued_operation_is_kept() {
        let mut orchestrator = OperationOrchestrator::new();
        orchestrator.begin_static_extraction().unwrap();

        orchestrator.queue_latest(QueuedOperation {
            app_id: "first".to_string(),
            window_id: None,
            trigger_text: "?fix".to_string(),
        });
        orchestrator.queue_latest(QueuedOperation {
            app_id: "second".to_string(),
            window_id: None,
            trigger_text: "?formal".to_string(),
        });

        assert_eq!(orchestrator.queued().unwrap().app_id, "second");
    }

    #[test]
    fn disabling_or_focus_change_cancels_active_and_queue() {
        let mut orchestrator = OperationOrchestrator::new();
        orchestrator.begin_static_extraction().unwrap();
        orchestrator.queue_latest(QueuedOperation {
            app_id: "queued".to_string(),
            window_id: None,
            trigger_text: "?fix".to_string(),
        });

        let queued = orchestrator.cancel(CancelReason::FocusChanged);

        assert_eq!(queued, None);
        assert_eq!(orchestrator.state(), OperationState::Idle);
        assert!(orchestrator.queued().is_none());
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut orchestrator = OperationOrchestrator::new();
        orchestrator.begin_static_extraction().unwrap();

        let result = orchestrator.begin_replacement();

        assert_eq!(
            result,
            Err(OrchestratorError::InvalidTransition {
                from: OperationState::Extracting,
                to: OperationState::Replacing
            })
        );
    }
}
