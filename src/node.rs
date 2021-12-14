use egui_nodes::{NodeArgs, NodeConstructor, PinArgs};

#[derive(Default)]
pub struct IdTracker {
    next: usize,
}

impl IdTracker {
    pub fn reserve(&mut self, count: usize) -> usize {
        let base = self.next;
        self.next += count;
        base
    }
}

pub enum Update {
    ControlFlow,
    Expression(naga::Handle<naga::Expression>),
}

#[derive(Debug)]
pub enum NodeType {
    Start,
    Return(Option<naga::Handle<naga::Expression>>),
    Expression(naga::Handle<naga::Expression>),
}

impl NodeType {
    pub fn name(&self, function: &naga::Function) -> &'static str {
        match *self {
            NodeType::Start => "Begin",
            NodeType::Return(_) => "Return",
            NodeType::Expression(handle) => match function.expressions[handle] {
                naga::Expression::Constant(_) => "Constant",
                naga::Expression::Binary { op, .. } => match op {
                    naga::BinaryOperator::Add => "Addition",
                    _ => todo!(),
                },
                _ => todo!(),
            },
        }
    }

    pub fn pin_count(&self, function: &naga::Function) -> usize {
        match *self {
            NodeType::Start => 1,     // 1 Control output
            NodeType::Return(_) => 2, // 1 Control input + 1 input
            NodeType::Expression(handle) => match function.expressions[handle] {
                naga::Expression::Constant(_) => 1,   // 1 Output
                naga::Expression::Binary { .. } => 3, // 2 Inputs + 1 Output
                _ => todo!(),
            },
        }
    }
}

pub struct Node {
    pub ty: NodeType,
    pub base_pin: usize,
}

impl Node {
    pub fn to_constructor<'a>(&self, id: usize, function: &naga::Function) -> NodeConstructor<'a> {
        let mut constructor = NodeConstructor::new(id, NodeArgs::default());

        let name = format!("{} [{:?}]", self.ty.name(function), self.ty);
        constructor.with_title(move |ui| ui.label(name));

        match self.ty {
            NodeType::Start => {
                constructor.with_output_attribute(self.base_pin, control_pin(), |ui| {
                    ui.label("Control flow")
                });
            }
            NodeType::Return(_) => {
                constructor.with_input_attribute(self.base_pin, control_pin(), |ui| {
                    ui.label("Control flow")
                });

                if function.result.is_some() {
                    constructor.with_input_attribute(self.base_pin + 1, PinArgs::default(), |ui| {
                        ui.label("value")
                    });
                }
            }
            NodeType::Expression(handle) => match function.expressions[handle] {
                naga::Expression::Constant(_) => {
                    constructor.with_output_attribute(self.base_pin, PinArgs::default(), |ui| {
                        ui.label("value")
                    });
                }
                naga::Expression::Binary { .. } => {
                    constructor
                        .with_input_attribute(self.base_pin, PinArgs::default(), |ui| {
                            ui.label("left")
                        })
                        .with_input_attribute(self.base_pin + 1, PinArgs::default(), |ui| {
                            ui.label("right")
                        })
                        .with_output_attribute(self.base_pin + 2, PinArgs::default(), |ui| {
                            ui.label("result")
                        });
                }
                _ => todo!(),
            },
        }

        constructor
    }

    pub fn update(&mut self, function: &mut naga::Function, pin: usize, update: Update) -> bool {
        let pin = pin - self.base_pin;
        match self.ty {
            NodeType::Expression(handle) => {
                let value = match update {
                    Update::ControlFlow => return false,
                    Update::Expression(handle) => handle,
                };

                match function.expressions[handle] {
                    naga::Expression::Constant(_) => return false,
                    naga::Expression::Binary {
                        ref mut left,
                        ref mut right,
                        ..
                    } => match pin {
                        0 => *left = value,
                        1 => *right = value,
                        _ => return false,
                    },
                    _ => todo!(),
                }
                true
            }
            NodeType::Start => false,
            NodeType::Return(ref mut value) => match (pin, update) {
                (0, Update::ControlFlow) => true,
                (1, Update::Expression(handle)) => {
                    *value = Some(handle);
                    true
                }
                _ => false,
            },
        }
    }
}

fn control_pin() -> PinArgs {
    PinArgs {
        shape: egui_nodes::PinShape::TriangleFilled,
        background: Some(egui::Color32::WHITE),
        ..Default::default()
    }
}
