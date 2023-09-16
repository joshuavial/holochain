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

mod tests {

    use std::sync::Arc;

    use super::*;
    use crate::{RecordActions, Share};

    //            G
    //           / \
    //          /   \
    //         C     F
    //        / \   / \
    //       A  B  D   E

    // struct A(Nested<A, B>, Nested<A, C>);
    #[derive(Debug)]
    struct C {
        a: Share<A>,
        b: Share<B>,
    }

    impl C {}

    // TODO: Root struct and impl can be macro generated
    struct Root(Share<C>);

    impl State<'static> for Root {
        type Action = <C as State<'static>>::Action;
        type Effect = <C as State<'static>>::Effect;

        fn transition(&mut self, action: Self::Action) -> Self::Effect {
            self.0.transition(action)
        }
    }

    // TODO: derive
    impl Root {
        pub fn new(c: C) -> Self {
            Self(Share::new(c))
        }

        pub fn a(&self) -> Nested<C, A> {
            Nested {
                child: self.0.read(|c| c.a.clone()),
                parent: self.0.clone(),
                xf: Cx::transformer_a(),
            }
        }

        pub fn b(&self) -> Nested<C, B> {
            Nested {
                child: self.0.read(|c| c.b.clone()),
                parent: self.0.clone(),
                xf: Cx::transformer_b(),
            }
        }
    }

    // struct C(Nested<C, F>, Nested<C, G>);
    #[derive(Debug)]
    struct A(bool);
    #[derive(Debug)]
    struct B(u8);
    // struct F(f64);
    // struct G(String);

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    enum Cx {
        A(Ax),
        B(Bx),
    }

    // TODO: derive
    impl Cx {
        fn transformer_a() -> Transformer<C, A> {
            Transformer {
                wrap_action: Arc::new(|a| Cx::A(a)),
                unwrap_effect: Arc::new(|c| match c {
                    Cf::A(a) => a,
                    _ => unreachable!(),
                }),
            }
        }
        fn transformer_b() -> Transformer<C, B> {
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
        let c = RecordActions::new(None, Root::new(c));

        c.a().transition(());
        c.b().transition(3);
    }
}
