use super::{InputEvent, KeystrokeBuffer, SyntheticInputGuard};
use crate::extraction::TextExtractor;
use crate::pipeline::{PipelineError, PipelineOutcome, TextTransformer, TransformationPipeline};
use crate::platform::{ForegroundAppProvider, OperationGate};
use crate::replacement::TextReplacer;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputControllerOutcome {
    IgnoredSynthetic,
    BufferUpdated(String),
    BufferCleared,
    Pipeline(PipelineOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputControllerError {
    Pipeline(PipelineError),
}

impl From<PipelineError> for InputControllerError {
    fn from(error: PipelineError) -> Self {
        Self::Pipeline(error)
    }
}

pub struct InputController<E, T, R, P> {
    buffer: KeystrokeBuffer,
    pipeline: TransformationPipeline<E, T, R>,
    foreground_provider: P,
    gate: OperationGate,
    synthetic_guard: SyntheticInputGuard,
}

impl<E, T, R, P> InputController<E, T, R, P>
where
    E: TextExtractor,
    T: TextTransformer,
    R: TextReplacer,
    P: ForegroundAppProvider,
{
    pub fn new(
        pipeline: TransformationPipeline<E, T, R>,
        foreground_provider: P,
        gate: OperationGate,
        synthetic_guard: SyntheticInputGuard,
    ) -> Self {
        Self {
            buffer: KeystrokeBuffer::default(),
            pipeline,
            foreground_provider,
            gate,
            synthetic_guard,
        }
    }

    pub fn handle_event(
        &mut self,
        event: InputEvent,
        now: Instant,
    ) -> Result<InputControllerOutcome, InputControllerError> {
        if self.synthetic_guard.is_suppressed(now) {
            return Ok(InputControllerOutcome::IgnoredSynthetic);
        }

        match event {
            InputEvent::Text(text) => {
                self.buffer.append(&text);
                let outcome = self.pipeline.process_foreground_buffer(
                    self.buffer.as_str(),
                    &mut self.foreground_provider,
                    &self.gate,
                )?;
                match outcome {
                    PipelineOutcome::NoMatch => Ok(InputControllerOutcome::BufferUpdated(
                        self.buffer.as_str().to_string(),
                    )),
                    PipelineOutcome::PendingDynamic => Ok(InputControllerOutcome::Pipeline(
                        PipelineOutcome::PendingDynamic,
                    )),
                    PipelineOutcome::Blocked(decision) => {
                        self.buffer.clear();
                        Ok(InputControllerOutcome::Pipeline(PipelineOutcome::Blocked(
                            decision,
                        )))
                    }
                    PipelineOutcome::Replaced { .. } => {
                        self.buffer.clear();
                        Ok(InputControllerOutcome::Pipeline(outcome))
                    }
                }
            }
            InputEvent::Backspace => {
                self.buffer.backspace();
                Ok(InputControllerOutcome::BufferUpdated(
                    self.buffer.as_str().to_string(),
                ))
            }
            InputEvent::Delete
            | InputEvent::Enter
            | InputEvent::Escape
            | InputEvent::Tab
            | InputEvent::Navigation(_)
            | InputEvent::MouseButton
            | InputEvent::Shortcut(_)
            | InputEvent::FocusChanged
            | InputEvent::SleepOrLock => {
                self.buffer.clear();
                Ok(InputControllerOutcome::BufferCleared)
            }
        }
    }

    pub fn buffer(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn into_parts(self) -> (TransformationPipeline<E, T, R>, P) {
        (self.pipeline, self.foreground_provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{CommandDefinition, CommandRegistry};
    use crate::extraction::BufferTextExtractor;
    use crate::platform::{
        ExclusionMatcher, ForegroundApp, OperationGate, StaticForegroundAppProvider,
    };
    use crate::replacement::NoopTextReplacer;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    struct FakeTransformer;

    impl TextTransformer for FakeTransformer {
        fn transform(
            &mut self,
            _command: &CommandDefinition,
            input: &str,
        ) -> Result<String, crate::pipeline::TransformError> {
            Ok(format!("fixed: {input}"))
        }
    }

    fn controller() -> InputController<
        BufferTextExtractor,
        FakeTransformer,
        NoopTextReplacer,
        StaticForegroundAppProvider,
    > {
        let pipeline = TransformationPipeline::new(
            CommandRegistry::new(),
            BufferTextExtractor,
            FakeTransformer,
            NoopTextReplacer::default(),
        );
        let foreground_provider = StaticForegroundAppProvider::new(ForegroundApp {
            app_id: "com.example.App".to_string(),
            window_id: None,
            display_name: None,
            secure_input: false,
            elevated: false,
        });
        let gate = OperationGate::new(true, ExclusionMatcher::new(Vec::new()));
        InputController::new(
            pipeline,
            foreground_provider,
            gate,
            SyntheticInputGuard::new(Duration::from_millis(250), Duration::from_secs(10)),
        )
    }

    #[test]
    fn text_events_update_buffer_until_trigger_matches() {
        let now = Instant::now();
        let mut controller = controller();

        let first = controller
            .handle_event(InputEvent::Text("hello ".to_string()), now)
            .unwrap();
        let second = controller
            .handle_event(InputEvent::Text("?fix".to_string()), now)
            .unwrap();

        assert_eq!(
            first,
            InputControllerOutcome::BufferUpdated("hello ".to_string())
        );
        assert!(matches!(
            second,
            InputControllerOutcome::Pipeline(PipelineOutcome::Replaced { .. })
        ));
        assert_eq!(controller.buffer(), "");
    }

    #[test]
    fn navigation_clears_buffer() {
        let now = Instant::now();
        let mut controller = controller();

        controller
            .handle_event(InputEvent::Text("hello".to_string()), now)
            .unwrap();
        let outcome = controller
            .handle_event(
                InputEvent::Navigation(super::super::NavigationKey::ArrowLeft),
                now,
            )
            .unwrap();

        assert_eq!(outcome, InputControllerOutcome::BufferCleared);
        assert_eq!(controller.buffer(), "");
    }

    #[test]
    fn synthetic_events_are_ignored() {
        let guard = SyntheticInputGuard::new(Duration::from_millis(250), Duration::from_secs(10));
        let pipeline = TransformationPipeline::new(
            CommandRegistry::new(),
            BufferTextExtractor,
            FakeTransformer,
            NoopTextReplacer::default(),
        );
        let foreground_provider = StaticForegroundAppProvider::new(ForegroundApp {
            app_id: "com.example.App".to_string(),
            window_id: None,
            display_name: None,
            secure_input: false,
            elevated: false,
        });
        let gate = OperationGate::new(true, ExclusionMatcher::new(Vec::new()));
        let now = Instant::now();
        let _token = guard.acquire(now);
        let mut controller = InputController::new(pipeline, foreground_provider, gate, guard);

        let outcome = controller
            .handle_event(InputEvent::Text("?fix".to_string()), now)
            .unwrap();

        assert_eq!(outcome, InputControllerOutcome::IgnoredSynthetic);
        assert_eq!(controller.buffer(), "");
    }

    #[test]
    fn blocked_foreground_context_clears_buffer() {
        let pipeline = TransformationPipeline::new(
            CommandRegistry::new(),
            BufferTextExtractor,
            FakeTransformer,
            NoopTextReplacer::default(),
        );
        let foreground_provider = StaticForegroundAppProvider::new(ForegroundApp {
            app_id: "1Password.exe".to_string(),
            window_id: None,
            display_name: None,
            secure_input: false,
            elevated: false,
        });
        let gate = OperationGate::new(true, ExclusionMatcher::from_config(&Default::default()));
        let mut controller = InputController::new(
            pipeline,
            foreground_provider,
            gate,
            SyntheticInputGuard::default(),
        );

        let outcome = controller
            .handle_event(InputEvent::Text("secret ?fix".to_string()), Instant::now())
            .unwrap();

        assert!(matches!(
            outcome,
            InputControllerOutcome::Pipeline(PipelineOutcome::Blocked(_))
        ));
        assert_eq!(controller.buffer(), "");
    }
}
