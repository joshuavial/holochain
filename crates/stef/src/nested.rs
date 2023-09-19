use std::sync::Arc;

use crate::{Share, State};

#[derive(Clone)]
struct Transformer<Parent, Child>
where
    Parent: State<'static>,
    Child: State<'static>,
{
    wrap_action: Arc<dyn Fn(Child::Action) -> Parent::Action>,
    unwrap_effect: Arc<dyn Fn(Parent::Effect) -> Child::Effect>,
}

#[derive(derive_more::Deref)]
pub struct Nested<Parent, Child>
where
    Parent: State<'static>,
    Child: State<'static>,
{
    #[deref]
    child: Share<Child>,
    parent: Share<Parent>,

    xf: Transformer<Parent, Child>,
}

impl<Parent, Child> std::fmt::Debug for Nested<Parent, Child>
where
    Parent: State<'static> + std::fmt::Debug,
    Child: State<'static> + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Nested")
            .field("parent", &self.parent)
            .field("child", &self.child)
            .finish()
    }
}

impl<Parent, Child> State<'static> for Nested<Parent, Child>
where
    Parent: State<'static>,
    Child: State<'static>,
{
    type Action = Child::Action;
    type Effect = Child::Effect;

    fn transition(&mut self, action: Self::Action) -> Self::Effect {
        (self.xf.unwrap_effect)(self.parent.transition((self.xf.wrap_action)(action)))
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use super::*;
    use crate::{Cassette, MemoryCassette, RecordActions, Share};

    //           Root
    //            |
    //            G
    //           / \
    //          /   \
    //         C     F
    //        / \   / \
    //       A  B  D   E

    // L1
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct A(bool);

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct B(u8);

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct D(f64);

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct E(String);

    // L2
    #[derive(Clone, Debug)]
    struct C {
        a: Share<A>,
        b: Share<B>,
    }

    #[derive(Clone, Debug)]
    struct F {
        d: Share<D>,
        e: Share<E>,
    }

    #[derive(Clone, Debug)]
    struct G {
        c: Share<C>,
        f: Share<F>,
    }

    type RootInner = RecordActions<C, MemoryCassette<C>>;

    // TODO: Root struct and impl can be macro generated
    #[derive(Clone, derive_more::Deref)]
    struct Root(Share<RootInner>);

    impl State<'static> for Root {
        type Action = <RootInner as State<'static>>::Action;
        type Effect = <RootInner as State<'static>>::Effect;

        fn transition(&mut self, action: Self::Action) -> Self::Effect {
            self.0.transition(action)
        }
    }

    // TODO: DERIVE
    impl Root {
        pub fn new(inner: RootInner) -> Self {
            Self(Share::new(inner))
        }

        pub fn a(&self) -> Nested<RootInner, A> {
            Nested {
                child: self.0.read(|c| c.a.clone()),
                parent: self.0.clone(),
                xf: Cx::transformer_a(),
            }
        }

        pub fn b(&self) -> Nested<RootInner, B> {
            Nested {
                child: self.0.read(|c| c.b.clone()),
                parent: self.0.clone(),
                xf: Cx::transformer_b(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    enum Cx {
        A(Ax),
        B(Bx),
    }

    // TODO: DERIVE
    impl Cx {
        fn transformer_a() -> Transformer<RootInner, A> {
            Transformer {
                wrap_action: Arc::new(|a| Cx::A(a)),
                unwrap_effect: Arc::new(|c| match c {
                    Cf::A(a) => a,
                    _ => unreachable!(),
                }),
            }
        }
        fn transformer_b() -> Transformer<RootInner, B> {
            Transformer {
                wrap_action: Arc::new(|b| Cx::B(b)),
                unwrap_effect: Arc::new(|c| match c {
                    Cf::B(b) => b,
                    _ => unreachable!(),
                }),
            }
        }
    }

    type Ax = ();
    type Bx = u8;

    #[derive(Debug, PartialEq, Eq)]
    enum Cf {
        A(Af),
        B(Bf),
    }
    type Af = bool;
    type Bf = u8;

    impl State<'static> for C {
        type Action = Cx;
        type Effect = Cf;

        fn transition(&mut self, ax: Self::Action) -> Self::Effect {
            match ax {
                Cx::A(a) => Cf::A(self.a.transition(a)),
                Cx::B(b) => Cf::B(self.b.transition(b)),
            }
        }
    }

    impl State<'static> for A {
        type Action = Ax;
        type Effect = Af;

        fn transition(&mut self, (): Self::Action) -> Self::Effect {
            self.0 = !self.0;
            self.0
        }
    }

    impl State<'static> for B {
        type Action = Bx;
        type Effect = Bf;

        fn transition(&mut self, amt: Self::Action) -> Self::Effect {
            self.0 += amt;
            self.0
        }
    }

    #[test]
    fn test_nested() {
        let a = A(false);
        let b = B(0);
        let c: C = C {
            a: Share::new(a),
            b: Share::new(b),
        };
        let cassette = MemoryCassette::new();
        let root = Root::new(RecordActions::new(cassette.clone(), c));

        let mut a1 = root.a();
        let mut a2 = root.a();
        let mut b1 = root.b();
        let mut b2 = root.b();

        a1.transition(());
        b1.transition(1);
        a1.transition(());
        a2.transition(());
        b2.transition(2);

        assert_eq!(
            cassette.retrieve_actions().unwrap(),
            vec![Cx::A(()), Cx::B(1), Cx::A(()), Cx::A(()), Cx::B(2),]
        );

        root.read(|s| {
            assert_eq!(s.a.get(), A(true));
            assert_eq!(s.b.get(), B(3));
        });
    }
}
