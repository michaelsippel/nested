
use {
    crate::{
        core::{ViewPort, OuterViewPort, Observer, port::UpdateTask, TypeTerm, TypeLadder, Context},
        terminal::{
            Terminal, TerminalAtom, TerminalCompositor, TerminalEditor,
            TerminalEditorResult, TerminalEvent, TerminalStyle, TerminalView,
            make_label
        },
        sequence::{SequenceView},
        tree_nav::{TreeNav, TerminalTreeEditor, TreeCursor, TreeNavResult},
        vec::{VecBuffer, MutableVecAccess},
        index::buffer::IndexBuffer,
        integer::PosIntEditor,
        string_editor::{StringEditor, CharEditor},
        list::{ListEditor, ListCursorMode, ListEditorStyle},
        product::{element::ProductEditorElement},
        make_editor::make_editor
    },
    cgmath::{Point2, Vector2},
    std::{sync::{Arc, RwLock}, ops::{Deref, DerefMut}},
    termion::event::{Event, Key},
};

pub struct ProductEditor {
    elements: VecBuffer<ProductEditorElement>,
    pub(super) n_indices: Vec<usize>,
    
    el_port: OuterViewPort<dyn SequenceView<Item = ProductEditorElement>>,
    el_view_port: OuterViewPort<dyn SequenceView<Item = OuterViewPort<dyn TerminalView>>>,

    pub(super) ctx: Arc<RwLock<Context>>,
    
    pub(super) cursor: Option<usize>,
    pub(super) depth: usize
}

impl ProductEditor {
    pub fn new(depth: usize, ctx: Arc<RwLock<Context>>) -> Self {
        let mut port = ViewPort::new();

        let el_port = port.outer().to_sequence();
        let el_view_port = el_port.map({
            let ctx = ctx.clone();
            move |e: &ProductEditorElement| { e.get_view(ctx.clone()) }
        });

        ProductEditor {
            elements: VecBuffer::new(port.inner()),
            el_port,
            el_view_port,
            n_indices: Vec::new(),

            ctx,

            cursor: None,
            depth
        }
    }

    pub fn with_t(mut self, t: &str) -> Self {
        self.elements.push(ProductEditorElement::T(t.to_string()));
        self
    }

    pub fn with_n(mut self, n: TypeLadder) -> Self {
        let elem_idx = self.elements.len();
        self.elements.push(ProductEditorElement::N{
            t: n,
            editor: None,
            select: false
        });
        self.n_indices.push(elem_idx);
        self
    }

    pub fn get_editor_element(&self, idx: usize) -> Option<ProductEditorElement> {
        if let Some(i) = self.n_indices.get(idx) {
            Some(self.elements.get(*i))
        } else {
            None
        }
    }

    pub fn get_editor_element_mut(&mut self, idx: usize) -> Option<MutableVecAccess<ProductEditorElement>> {
        if let Some(i) = self.n_indices.get(idx) {
            Some(self.elements.get_mut(*i))
        } else {
            None
        }
    }

    pub fn get_cur_element(&self) -> Option<ProductEditorElement> {
        self.get_editor_element(self.cursor?)
    }

    pub fn get_cur_element_mut(&mut self) -> Option<MutableVecAccess<ProductEditorElement>> {
        self.get_editor_element_mut(self.cursor?)
    }

    pub fn get_editor(&self, idx: usize) -> Option<Arc<RwLock<dyn TerminalTreeEditor + Send + Sync>>> {
        if let Some(ProductEditorElement::N{ t: _, editor, select: _ }) = self.get_editor_element(idx) {
            editor
        } else {
            None
        }
    }
    
    pub fn get_cur_editor(&self) -> Option<Arc<RwLock<dyn TerminalTreeEditor + Send + Sync>>> {
        self.get_editor(self.cursor?)
    }

    pub fn set_leaf_mode(&mut self, mode: ListCursorMode) {
        let mut c = self.get_cursor();
        c.leaf_mode = mode;
        self.goto(c);
    }
}

impl TerminalEditor for ProductEditor {
    fn get_term_view(&self) -> OuterViewPort<dyn TerminalView> {
        self.el_view_port.to_grid_horizontal().flatten()
    }

    fn handle_terminal_event(&mut self, event: &TerminalEvent) -> TerminalEditorResult {
        if let Some(ProductEditorElement::N{ t, editor, select }) = self.get_cur_element_mut().as_deref_mut() {
            *select = true;
            if let Some(e) = editor.clone() {
                match e.clone().write().unwrap().handle_terminal_event(event) {
                    TerminalEditorResult::Exit =>
                        match event {
                            TerminalEvent::Input(Event::Key(Key::Backspace)) => {
                                *editor = None;
                                TerminalEditorResult::Continue
                            }
                            _ => {
                                drop(e);
                                match self.nexd() {
                                    TreeNavResult::Continue => TerminalEditorResult::Continue,
                                    TreeNavResult::Exit => TerminalEditorResult::Exit
                                }
                            }
                        },
                    TerminalEditorResult::Continue =>
                    TerminalEditorResult::Continue
                }
            } else {
                let e = make_editor(self.ctx.clone(), t, self.depth+1);
                *editor = Some(e.clone());
                e.write().unwrap().dn();
                let x = e.write().unwrap().handle_terminal_event(event);
                x
            }
        } else {
            TerminalEditorResult::Exit
        }
    }
}
