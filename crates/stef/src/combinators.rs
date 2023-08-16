//! Wrapper types to add functionality and behavior to a [`State`]

use std::{collections::VecDeque, marker::PhantomData};

use super::*;

/// Every time an Effect is generated from this State, just store it in a Vec.
/// The stored Effects can be accessed at any time later.
/// Converts the State's Effect type to `()`
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct StoreEffects<S: State> {
    #[deref]
    #[deref_mut]
    state: S,
    effects: VecDeque<S::Effect>,
}

pub enum Act<Ax: Action> {
    Push(Ax),
    Pop,
}

impl<S: State> State for StoreEffects<S> {
    type Action = S::Action;
    type Effect = ();

    fn transition(&mut self, t: Self::Action) -> Self::Effect {
        let eff = self.state.transition(t);
        self.effects.push_back(eff);
    }
}

impl<S: State> StoreEffects<S> {
    /// Constructor
    pub fn new(state: S, capacity: usize) -> Self {
        Self {
            state,
            effects: VecDeque::new(),
        }
    }

    /// Accessor for the stored effects
    pub fn effects(&self) -> &VecDeque<S::Effect> {
        &self.effects
    }

    /// Drain and return all effects.
    /// Useful if you want to defer execution of some effects.
    pub fn drain_effects(&mut self) -> Vec<S::Effect> {
        std::mem::replace(&mut self.effects, VecDeque::new())
            .into_iter()
            .collect()
    }
}

/// Immediately run any generated Effects.
/// The new Effect type for the modified State will be whatever
/// the return value of the runner function is.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct RunEffects<S: State, Ret, Runner> {
    #[deref]
    #[deref_mut]
    state: S,
    runner: Runner,
    _phantom: PhantomData<Ret>,
}

impl<S: State + Default, Ret: Effect, Runner: Fn(S::Effect) -> Ret> ParamState
    for RunEffects<S, Ret, Runner>
{
    type Action = S::Action;
    type Effect = Ret;
    type State = S;
    type Params = Runner;

    fn initial(runner: Runner) -> Self {
        Self {
            state: S::default(),
            runner,
            _phantom: PhantomData,
        }
    }

    fn partition(&mut self) -> (&mut Self::State, &Self::Params) {
        (&mut self.state, &self.runner)
    }

    fn update(
        state: &mut Self::State,
        runner: &Self::Params,
        action: Self::Action,
    ) -> Self::Effect {
        (runner)(state.transition(action))
    }
}

impl<S: State, Ret, Runner: Fn(S::Effect) -> Ret> RunEffects<S, Ret, Runner> {
    /// Constructor
    pub fn new(state: S, runner: Runner) -> Self {
        Self {
            state,
            runner,
            _phantom: PhantomData,
        }
    }
}
