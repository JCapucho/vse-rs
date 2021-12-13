use egui_nodes::LinkArgs;
use node::{IdTracker, Node};

mod node;

enum FunctionHandle {
    Function(naga::Handle<naga::Function>),
    EntryPoint(usize),
}

struct FunctionData {
    handle: FunctionHandle,
    nodes: Vec<Node>,
    links: Vec<(usize, usize)>,
}

pub struct Editor {
    context: egui_nodes::Context,
    functions: Vec<FunctionData>,
    module: naga::Module,

    pin_tracker: IdTracker,
    expression_to_pin: naga::FastHashMap<naga::Handle<naga::Expression>, usize>,

    select_function: usize,
}

impl Editor {
    pub fn load_module(&mut self, module: naga::Module) {
        self.functions.clear();
        self.expression_to_pin.clear();
        self.pin_tracker.clear();
        self.select_function = 0;

        for (i, ep) in module.entry_points.iter().enumerate() {
            let handle = FunctionHandle::EntryPoint(i);
            self.process_function(handle, &ep.function);
        }

        for (handle, function) in module.functions.iter() {
            let handle = FunctionHandle::Function(handle);
            self.process_function(handle, function);
        }

        self.module = module;

        if self.functions.is_empty() {
            let function_handle = self
                .module
                .functions
                .append(naga::Function::default(), naga::Span::default());

            let function = FunctionData {
                handle: FunctionHandle::Function(function_handle),
                nodes: Vec::new(),
                links: Vec::new(),
            };

            self.functions.push(function)
        }
    }

    fn process_function(&mut self, handle: FunctionHandle, function: &naga::Function) {
        let mut nodes = Vec::with_capacity(function.expressions.len());
        let mut links = Vec::with_capacity(function.expressions.len() * 4);

        for (handle, expr) in function.expressions.iter() {
            let ty = node::NodeType::Expression(handle);
            let pin_count = ty.pin_count(function);
            let base_pin = self.pin_tracker.reserve(pin_count);

            self.expression_to_pin
                .insert(handle, base_pin + pin_count - 1);

            nodes.push(Node { ty, base_pin });

            match *expr {
                naga::Expression::Constant(_) => {}
                naga::Expression::Binary { left, right, .. } => {
                    let left_pin = self.expression_to_pin[&left];
                    let right_pin = self.expression_to_pin[&right];
                    links.reserve(2);
                    links.push((base_pin, left_pin));
                    links.push((base_pin + 1, right_pin));
                }
                _ => todo!(),
            }
        }

        self.functions.push(FunctionData {
            handle,
            nodes,
            links,
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let Editor {
            ref mut functions,
            ref mut module,
            ref mut context,
            ..
        } = self;
        let current_function = &mut functions[self.select_function];

        let side_panel = egui::SidePanel::left("vse_side_panel").show_inside(ui, |ui| {
            let name = match current_function.handle {
                FunctionHandle::Function(handle) => module.functions[handle]
                    .name
                    .get_or_insert_with(|| String::new()),
                FunctionHandle::EntryPoint(idx) => &mut module.entry_points[idx].name,
            };

            ui.text_edit_singleline(name);
        });

        let function = match current_function.handle {
            FunctionHandle::Function(handle) => &mut module.functions[handle],
            FunctionHandle::EntryPoint(idx) => &mut module.entry_points[idx].function,
        };

        let editor = egui::CentralPanel::default().show_inside(ui, |ui| {
            let links_iter = current_function
                .links
                .iter()
                .enumerate()
                .map(|(i, (pin_start, pin_end))| (i, *pin_start, *pin_end, LinkArgs::default()));
            let nodes_iter = current_function
                .nodes
                .iter()
                .enumerate()
                .map(|(i, node)| node.to_constructor(i, function));

            context.show(nodes_iter, links_iter, ui);

            if let Some((pin_start, node_start, pin_end, node_end, _)) = context.link_created_node()
            {
                let value = match current_function.nodes[node_start].ty {
                    node::NodeType::Expression(handle) => handle,
                };

                let node = &current_function.nodes[node_end];
                let pin = pin_end - node.base_pin;
                match node.ty {
                    node::NodeType::Expression(handle) => match function.expressions[handle] {
                        naga::Expression::Constant(_) => panic!("Node has no inputs"),
                        naga::Expression::Binary {
                            ref mut left,
                            ref mut right,
                            ..
                        } => match pin {
                            0 => *left = value,
                            1 => *right = value,
                            _ => unreachable!(),
                        },
                        _ => todo!(),
                    },
                }

                current_function.links.push((pin_start, pin_end));
            }

            if let Some(idx) = context.link_destroyed() {
                current_function.links.remove(idx);
            }
        });

        side_panel.response | editor.response
    }

    pub fn module(&self) -> &naga::Module {
        &self.module
    }
}

impl Default for Editor {
    fn default() -> Self {
        let mut module = naga::Module::default();

        let function_handle = module
            .functions
            .append(naga::Function::default(), naga::Span::default());

        let function = FunctionData {
            handle: FunctionHandle::Function(function_handle),
            nodes: Vec::new(),
            links: Vec::new(),
        };

        Editor {
            context: Default::default(),
            functions: vec![function],
            module,
            pin_tracker: Default::default(),
            expression_to_pin: Default::default(),
            select_function: Default::default(),
        }
    }
}
