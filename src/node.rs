use egui_nodes::{NodeArgs, NodeConstructor, PinArgs};

#[derive(Default)]
pub struct IdTracker {
    next: usize,
}

impl IdTracker {
    pub fn clear(&mut self) {
        self.next = 0
    }

    pub fn reserve(&mut self, count: usize) -> usize {
        let base = self.next;
        self.next += count;
        base
    }
}

#[derive(Debug)]
pub enum NodeType {
    Expression(naga::Handle<naga::Expression>),
}

impl NodeType {
    pub fn name(&self, function: &naga::Function) -> &'static str {
        match *self {
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
}
