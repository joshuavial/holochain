use std::{
    fs::File,
    io::{Read, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{de::DeserializeOwned, Serialize};

use crate::*;

pub trait ActionRecorder<S: State<'static>> {
    fn initialize(&self) -> anyhow::Result<()>;

    fn record_action(&self, action: &S::Action) -> anyhow::Result<()>;

    // TODO: buffer, or just provide playback_actions
    fn retrieve_actions(&self) -> anyhow::Result<Vec<S::Action>>;

    fn playback_actions(&self, state: &mut S) -> anyhow::Result<Vec<S::Effect>> {
        Ok(self
            .retrieve_actions()?
            .into_iter()
            .map(|action| state.transition(action))
            .collect())
    }
}

pub struct FileActionRecorder<S> {
    path: PathBuf,
    state: PhantomData<S>,
}

impl<S> From<PathBuf> for FileActionRecorder<S> {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            state: PhantomData,
        }
    }
}

impl<S: State<'static>> ActionRecorder<S> for FileActionRecorder<S>
where
    S::Action: Serialize + DeserializeOwned,
{
    fn initialize(&self) -> anyhow::Result<()> {
        File::options()
            .write(true)
            .create_new(true)
            .open(&self.path)?;
        Ok(())
    }

    fn record_action(&self, action: &S::Action) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(action)?;
        let mut f = File::options().append(true).open(&self.path)?;
        let len = bytes.len() as u32;
        f.write(&len.to_le_bytes())?;
        f.write(&bytes)?;
        Ok(())
    }

    fn retrieve_actions(&self) -> anyhow::Result<Vec<S::Action>> {
        let mut f = File::open(&self.path)?;
        let mut lbuf = [0; 4];
        let mut abuf = Vec::new();
        let mut actions = Vec::new();
        loop {
            match f.read(&mut lbuf) {
                Ok(0) => return Ok(actions),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        {
                            return Ok(actions);
                        }
                    } else {
                        return Err(e.into());
                    }
                }
                Ok(_) => {
                    let len = u32::from_le_bytes(lbuf);
                    abuf.resize(len as usize, 0);
                    f.read(&mut abuf)?;
                    actions.push(rmp_serde::from_slice(&abuf).unwrap());
                }
            }
        }
    }
}

/// Shared access to FetchPoolState
#[derive(Clone, Debug, derive_more::Deref)]
pub struct RecordActions<S, R = FileActionRecorder<S>> {
    #[deref]
    state: S,
    recorder: Option<Arc<R>>,
}

impl<S> State<'static> for RecordActions<S>
where
    S: State<'static>,
    S::Action: Serialize + DeserializeOwned,
{
    type Action = S::Action;
    type Effect = S::Effect;

    fn transition(&mut self, action: Self::Action) -> Self::Effect {
        if let Some(r) = self.recorder.as_ref() {
            r.record_action(&action).unwrap()
        }
        self.state.transition(action)
    }
}

impl<S, R> RecordActions<S, R>
where
    S: State<'static>,
    S::Action: Serialize + DeserializeOwned,
    R: ActionRecorder<S>,
{
    pub fn new(recorder: Option<impl Into<R>>, state: S) -> Self {
        let recorder = recorder.map(Into::into);
        if let Some(recorder) = recorder.as_ref() {
            recorder.initialize().unwrap()
        }
        Self {
            recorder: recorder.map(Arc::new),
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
    let mut rec = RecordActions::new(Some(path.clone()), ());
    rec.transition(());
    rec.transition(());
    rec.transition(());
    let actions: Vec<()> = RecordActions::<()>::retrieve_actions(path).unwrap();
    assert_eq!(actions, vec![(), (), ()]);
}
