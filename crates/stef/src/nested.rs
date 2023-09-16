use crate::{Share, State};

pub struct Nested<Root, This>
where
    Root: State<'static>,
    This: State<'static>,
{
    root: Root,
    this: This,
    wrap: Box<dyn Fn(This::Action) -> Root::Action>,
    unwrap: Box<dyn Fn(Root::Effect) -> This::Effect>,
}

impl<Root, This> State<'static> for Nested<Root, This>
where
    Root: State<'static>,
    This: State<'static>,
{
    type Action = This::Action;
    type Effect = This::Effect;

    fn transition(&mut self, action: Self::Action) -> Self::Effect {
        (self.unwrap)(self.root.transition((self.wrap)(action)))
    }
}

mod tests {

    use std::sync::Arc;

    use super::*;
    use crate::Share;

    //        G
    //       / \
    //      /   \
    //     C     F
    //    / \   / \
    //   A  B  D   E

    // struct A(Nested<A, B>, Nested<A, C>);
    struct C {
        a: Nested<Share<C>, A>,
        b: Nested<Share<C>, B>,
    }
    // struct C(Nested<C, F>, Nested<C, G>);
    struct A(bool);
    struct B(u8);
    // struct F(f64);
    // struct G(String);

    enum Cx {
        A(Ax),
        B(Bx),
    }
    type Ax = ();
    type Bx = u8;

    #[derive(PartialEq, Eq)]
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
    fn test_nested() {}
}
