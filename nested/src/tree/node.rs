use {
    std::sync::{Arc, RwLock},
    cgmath::Vector2,
    crate::{
        core::{ViewPort, OuterViewPort, AnyOuterViewPort, context::ReprTree, Context},
        singleton::{SingletonView, SingletonBuffer},
        sequence::SequenceView,
        terminal::{TerminalView, TerminalEvent, TerminalEditor, TerminalEditorResult},
        diagnostics::{Diagnostics, Message},
        tree::{TreeNav, TreeCursor, TreeNavResult},
        Commander, ObjCommander,
        Nested
    },
};

pub struct NestedNode {
    ctx: Option<Arc<RwLock<Context>>>,
    view: Option<OuterViewPort<dyn TerminalView>>,
    diag: Option<OuterViewPort<dyn SequenceView<Item = Message>>>,
    cmd: Option<Arc<RwLock<dyn ObjCommander + Send + Sync>>>,
    tree_nav: Option<Arc<RwLock<dyn TreeNav + Send + Sync>>>,
}

impl ObjCommander for NestedNode {
    fn send_cmd_obj(&mut self, cmd_obj: Arc<RwLock<ReprTree>>) {
        if let Some(cmd) = self.cmd.as_ref() {
            cmd.write().unwrap().send_cmd_obj(cmd_obj);
        }
    }
}

impl TerminalEditor for NestedNode {
    fn get_term_view(&self) -> OuterViewPort<dyn TerminalView> {
        self.get_view()
    }

    fn handle_terminal_event(&mut self, event: &TerminalEvent) -> TerminalEditorResult {
        let buf = SingletonBuffer::new(event.clone());

        if let (Some(cmd),Some(ctx)) = (self.cmd.as_ref(),self.ctx.as_ref()) {
            cmd.write().unwrap().send_cmd_obj(
                ReprTree::new_leaf(
                    ctx.read().unwrap().type_term_from_str("( TerminalEvent )").unwrap(),
                    AnyOuterViewPort::from(buf.get_port())
                ));
        }

        TerminalEditorResult::Continue
    }
}


impl TreeNav for NestedNode {
    fn get_cursor(&self) -> TreeCursor {
        if let Some(tn) = self.tree_nav.as_ref() {
            tn.read().unwrap().get_cursor()
        } else {
            TreeCursor::default()
        }
    }

    fn get_cursor_warp(&self) -> TreeCursor {
        if let Some(tn) = self.tree_nav.as_ref() {
            tn.read().unwrap().get_cursor_warp()
        } else {
            TreeCursor::default()
        }
    }

    fn get_max_depth(&self) -> usize {
        0
    }

    fn goby(&mut self, direction: Vector2<isize>) -> TreeNavResult {
        if let Some(tn) = self.tree_nav.as_ref() {
            tn.write().unwrap().goby(direction)
        } else {
            TreeNavResult::Exit
        }
    }

    fn goto(&mut self, new_cursor: TreeCursor) -> TreeNavResult {
        if let Some(tn) = self.tree_nav.as_ref() {
            tn.write().unwrap().goto(new_cursor)
        } else {
            TreeNavResult::Exit
        }
    }
}

impl Diagnostics for NestedNode {}
impl Nested for NestedNode {}

impl NestedNode {
    pub fn new() -> Self {
        NestedNode {
            ctx: None,
            view: None,
            diag: None,
            cmd: None,
            tree_nav: None
        }
    }

    pub fn set_ctx(mut self, ctx: Arc<RwLock<Context>>) -> Self {
        self.ctx = Some(ctx);
        self
    }

    pub fn set_view(mut self, view: OuterViewPort<dyn TerminalView>) -> Self {
        self.view = Some(view);
        self
    }

    pub fn with_cmd(mut self, cmd: Arc<RwLock<dyn ObjCommander + Send + Sync>>) -> Self {
        self.cmd = Some(cmd);
        self
    }

    pub fn get_view(&self) -> OuterViewPort<dyn TerminalView> {
        self.view.clone().unwrap_or(ViewPort::new().into_outer())
    }
}

