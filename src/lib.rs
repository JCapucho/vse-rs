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
    links: Vec<(usize, usize, bool)>,

    pin_tracker: IdTracker,
    expression_to_pin: naga::FastHashMap<naga::Handle<naga::Expression>, usize>,
}

pub struct Editor {
    context: egui_nodes::Context,
    functions: Vec<FunctionData>,
    module: naga::Module,

    select_function: usize,
    menu: Option<egui::Pos2>,
}

impl Editor {
    pub fn load_module(&mut self, module: naga::Module) {
        self.functions.clear();
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
            self.add_function()
        }
    }

    fn process_function(&mut self, handle: FunctionHandle, function: &naga::Function) {
        let mut nodes = Vec::with_capacity(function.expressions.len());
        let mut links = Vec::with_capacity(function.expressions.len() * 4);
        let mut expression_to_pin = naga::FastHashMap::with_capacity_and_hasher(
            function.expressions.len(),
            Default::default(),
        );
        let mut pin_tracker = IdTracker::default();

        for (handle, expr) in function.expressions.iter() {
            let ty = node::NodeType::Expression(handle);
            let pin_count = ty.pin_count(function);
            let base_pin = pin_tracker.reserve(pin_count);

            expression_to_pin.insert(handle, base_pin + pin_count - 1);

            nodes.push(Node { ty, base_pin });

            match *expr {
                naga::Expression::Constant(_) => {}
                naga::Expression::Binary { left, right, .. } => {
                    let left_pin = expression_to_pin[&left];
                    let right_pin = expression_to_pin[&right];
                    links.reserve(2);
                    links.push((base_pin, left_pin, false));
                    links.push((base_pin + 1, right_pin, false));
                }
                _ => todo!(),
            }
        }

        let last_link = pin_tracker.reserve(1);
        nodes.push(Node {
            ty: node::NodeType::Start,
            base_pin: last_link,
        });

        for stmt in function.body.iter() {
            match *stmt {
                naga::Statement::Emit(_) => {}
                naga::Statement::Return { value } => {
                    let ty = node::NodeType::Return(value);
                    let base_pin = pin_tracker.reserve(ty.pin_count(function));

                    nodes.push(Node { ty, base_pin });

                    links.push((last_link, base_pin, true));
                    if let Some(expr) = value {
                        let res_pin = expression_to_pin[&expr];
                        links.push((res_pin, base_pin + 1, false))
                    }
                }
                _ => todo!(),
            }
        }

        self.functions.push(FunctionData {
            handle,
            nodes,
            links,
            pin_tracker,
            expression_to_pin,
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let Editor {
            ref mut functions,
            ref mut module,
            ref mut context,
            ref mut menu,
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
            let links_iter = current_function.links.iter().enumerate().map(
                |(i, (pin_start, pin_end, control_flow))| {
                    let args = if *control_flow {
                        LinkArgs {
                            base: Some(egui::Color32::WHITE),
                            ..Default::default()
                        }
                    } else {
                        LinkArgs::default()
                    };
                    (i, *pin_start, *pin_end, args)
                },
            );
            let nodes_iter = current_function
                .nodes
                .iter()
                .enumerate()
                .map(|(i, node)| node.to_constructor(i, function));

            let response = context.show(nodes_iter, links_iter, ui);

            if let Some((pin_start, node_start, pin_end, node_end, _)) = context.link_created_node()
            {
                let (update, control_flow) = match current_function.nodes[node_start].ty {
                    node::NodeType::Expression(handle) => (node::Update::Expression(handle), false),
                    _ => (node::Update::ControlFlow, true),
                };

                if current_function.nodes[node_end].update(function, pin_end, update) {
                    current_function
                        .links
                        .push((pin_start, pin_end, control_flow));
                }
            }

            if let Some(idx) = context.link_destroyed() {
                current_function.links.remove(idx);
            }

            if response.secondary_clicked() {
                let pos = ui
                    .input()
                    .pointer
                    .hover_pos()
                    .unwrap_or_else(|| response.rect.center());

                *menu = Some(pos);
            }

            response
        });

        let mut response = side_panel.response | editor.response;

        if let Some(pos) = *menu {
            response |= egui::Area::new("vse_cursor_menu")
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label("Add node");
                        if ui.button("Addition").clicked() {
                            let handle = function.expressions.append(
                                naga::Expression::Binary {
                                    op: naga::BinaryOperator::Add,
                                    left: naga::Handle::from_usize(1),
                                    right: naga::Handle::from_usize(1),
                                },
                                naga::Span::default(),
                            );
                            current_function.nodes.push(Node {
                                ty: node::NodeType::Expression(handle),
                                base_pin: current_function.pin_tracker.reserve(3),
                            });
                            *menu = None;
                        }
                    })
                })
                .response
        }

        response
    }

    pub fn module(&self) -> &naga::Module {
        &self.module
    }

    fn add_function(&mut self) {
        let function_handle = self
            .module
            .functions
            .append(naga::Function::default(), naga::Span::default());

        let mut pin_tracker = IdTracker::default();

        let function = FunctionData {
            handle: FunctionHandle::Function(function_handle),
            nodes: vec![Node {
                ty: node::NodeType::Start,
                base_pin: pin_tracker.reserve(1),
            }],
            links: Vec::new(),
            pin_tracker,
            expression_to_pin: Default::default(),
        };

        self.functions.push(function)
    }
}

impl Default for Editor {
    fn default() -> Self {
        let mut this = Editor {
            context: Default::default(),
            functions: Default::default(),
            module: Default::default(),
            select_function: Default::default(),
            menu: Default::default(),
        };

        this.add_function();

        this
    }
}
