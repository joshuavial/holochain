use stef::Share;

pub enum Button {
    Lo,
    Mid,
    Hi,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ButtonEffect;

pub struct Slider(u8);

#[derive(stef_derive::State)]
struct Panel {
    button: Share<Button>,
    slider: Share<Slider>,
}

impl stef::State<'static> for Button {
    type Action = ();
    type Effect = ButtonEffect;

    fn transition(&mut self, (): Self::Action) -> Self::Effect {
        *self = match self {
            Button::Lo => Button::Mid,
            Button::Mid => Button::Hi,
            Button::Hi => Button::Lo,
        };
        ButtonEffect
    }
}

#[stef::state]
impl stef::State<'static> for Slider {
    type Action = SliderAction;
    type Effect = ();

    fn set(&mut self, val: u8) -> () {
        self.0 = val;
    }
}
