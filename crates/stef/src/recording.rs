use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};

use crate::*;

/// Shared access to FetchPoolState
#[derive(Clone, Debug, derive_more::Deref)]
pub struct RecordActions<S, R = FileActionRecorder<S>> {
    #[deref]
    state: S,
    recorder: Arc<R>,
}

impl<S, R> State<'static> for RecordActions<S, R>
where
    S: State<'static>,
    S::Action: Serialize + DeserializeOwned,
    R: ActionRecorder<S>,
{
    type Action = S::Action;
    type Effect = S::Effect;

    fn transition(&mut self, action: Self::Action) -> Self::Effect {
        self.recorder.record_action(&action).unwrap();
        self.state.transition(action)
    }
}

impl<S, R> RecordActions<S, R>
where
    S: State<'static>,
    S::Action: Serialize + DeserializeOwned,
    R: ActionRecorder<S>,
{
    pub fn new(recorder: R, state: S) -> Self {
        recorder.initialize().unwrap();
        Self {
            recorder: Arc::new(recorder),
            state,
        }
    }

    pub fn retrieve_actions(recorder: impl Into<R>) -> anyhow::Result<Vec<S::Action>> {
        let r: R = recorder.into();
        r.retrieve_actions()
    }
}

#[test]
fn action_recording_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("actions.stef");
    let mut rec = RecordActions::new(FileActionRecorder::from(path.clone()), ());
    rec.transition(());
    rec.transition(());
    rec.transition(());
    let actions: Vec<()> = RecordActions::<()>::retrieve_actions(path).unwrap();
    assert_eq!(actions, vec![(), (), ()]);
}
