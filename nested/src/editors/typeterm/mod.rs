mod ctx;

pub use ctx::init_ctx;

use {
    r3vi::{
        buffer::{singleton::*, vec::*},
        view::{singleton::*, sequence::*, OuterViewPort}
    },
    crate::{
        type_system::{Context, TypeID, TypeTerm, ReprTree},
        editors::{list::{ListCursorMode, ListEditor, ListCmd}},
        tree::{NestedNode, TreeNav, TreeNavResult, TreeCursor},
        commander::ObjCommander
    },
    std::{sync::{Arc, RwLock, Mutex}, any::Any},
    cgmath::{Vector2}
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum State {
    Any,
    Num,
    Char,
    AnySymbol,
    FunSymbol,
    VarSymbol,
    App,
    Ladder,
}

pub struct TypeTermEditor {
    ctx: Arc<RwLock<Context>>,
    data: Arc<RwLock<ReprTree>>,

    // references to Node pointing to TypeTermEditor
    close_char: SingletonBuffer<Option<char>>,
    spillbuf: Arc<RwLock<Vec<Arc<RwLock<NestedNode>>>>>,

    state: State,
    cur_node: SingletonBuffer< NestedNode >
}

impl TypeTermEditor {
    pub fn from_type_term(ctx: Arc<RwLock<Context>>, depth: usize, term: &TypeTerm) -> NestedNode {
        let mut node = TypeTermEditor::new_node(ctx.clone(), depth);
        node.goto(TreeCursor::home());

        match term {
            TypeTerm::TypeID( tyid ) => {
                let editor = node.get_edit::<TypeTermEditor>().expect("typ term edit");
                editor.write().unwrap().set_state(match tyid {
                    TypeID::Fun(_) => State::FunSymbol,
                    TypeID::Var(_) => State::VarSymbol
                });

                let typename = ctx.read().unwrap().get_typename(&tyid).unwrap_or("UNNAMED TYPE".into());
                for x in typename.chars()
                {
                    node.send_cmd_obj(
                        ReprTree::from_char( &ctx, x )
                    );
                }
            },

            TypeTerm::App( args ) => {
                let editor = node.get_edit::<TypeTermEditor>().expect("typ term edit");
                editor.write().unwrap().set_state( State::App );

                let parent_ctx = editor.read().unwrap().cur_node.get().ctx.clone();

                for x in args.iter() {                    
                    let arg_node = TypeTermEditor::from_type_term( parent_ctx.clone(), depth+1, x );

                    node.send_cmd_obj(
                        ReprTree::new_leaf(
                            (&ctx, "( NestedNode )"),
                            SingletonBuffer::new(arg_node).get_port().into()
                        )
                    );
                }
            }

            TypeTerm::Ladder( args ) => {
                let editor = node.get_edit::<TypeTermEditor>().expect("typ term edit");
                editor.write().unwrap().set_state( State::Ladder );

                let parent_ctx = editor.read().unwrap().cur_node.get().ctx.clone();

                for x in args.iter() {
                    let arg_node = TypeTermEditor::from_type_term( parent_ctx.clone(), depth+1, x );

                    node.send_cmd_obj(
                        ReprTree::new_leaf(
                            (&ctx, "( NestedNode )"),
                            SingletonBuffer::new(arg_node).get_port().into()
                        )
                    );
                }
            }

            TypeTerm::Num( n ) => {
                let editor = node.get_edit::<TypeTermEditor>().expect("typ term edit");

                let parent_ctx = editor.read().unwrap().cur_node.get().ctx.clone();

                let int_edit = crate::editors::integer::PosIntEditor::from_u64(parent_ctx, 10, *n as u64);
                let node = int_edit.into_node();
                editor.write().unwrap().cur_node.set(node);
                editor.write().unwrap().state = State::Num;
            }

            TypeTerm::Char( c ) => {
                let editor = node.get_edit::<TypeTermEditor>().expect("typ term edit");

                editor.write().unwrap().set_state( State::Char );
                editor.write().unwrap().send_cmd_obj(ReprTree::from_char(&ctx, *c));
            }
            
            _ => {}
        }

        node.goto(TreeCursor::none());
        node
    }
    
    fn set_state(&mut self, new_state: State) {
        eprintln!("TypeEdit: set state to {:?}", new_state);
        
        let old_node = self.cur_node.get();

        let mut node = match new_state {
            State::App => {
                Context::make_node( &self.ctx, (&self.ctx, "( List Type )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::App )").into() )
            }
            State::Ladder => {
                Context::make_node( &self.ctx, (&self.ctx, "( List Type )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::Ladder )").into() )
            }
            State::AnySymbol => {
                Context::make_node( &self.ctx, (&self.ctx, "( List Char )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::Sym )").into() )
            },
            State::FunSymbol => {
                Context::make_node( &self.ctx, (&self.ctx, "( List Char )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::Sym::Fun )").into() )
            },
            State::VarSymbol => {
                Context::make_node( &self.ctx, (&self.ctx, "( List Char )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::Sym::Var )").into() )
            }
            State::Num => {
                crate::editors::integer::PosIntEditor::new(self.ctx.clone(), 10)
                    .into_node()
                    .morph( (&self.ctx, "( Type::Lit::Num )").into() )
            }
            State::Char => {
                Context::make_node( &self.ctx, (&self.ctx, "( Char )").into(), 0 ).unwrap()
                    .morph( (&self.ctx, "( Type::Lit::Char )").into() )
            }
            _ => {
                old_node
            }
        };

        node.goto(TreeCursor::home());

        let editor = node.editor.get();
        self.close_char.set(node.close_char.get());
        self.cur_node.set(node);
        self.state = new_state;
    }

    pub fn new_node(ctx: Arc<RwLock<Context>>, depth: usize) -> NestedNode {
        let ctx : Arc<RwLock<Context>> = Arc::new(RwLock::new(Context::with_parent(Some(ctx))));
        ctx.write().unwrap().meta_chars.push('~');

        let mut symb_node = Context::make_node( &ctx, (&ctx, "( List Char )").into(), 0 ).unwrap();
        symb_node = symb_node.morph( (&ctx, "( Type::Sym )").into() );

        Self::with_node(
            ctx.clone(),
            depth,
            symb_node,
            State::Any
        )
    }

    fn with_node(ctx: Arc<RwLock<Context>>, depth: usize, node: NestedNode, state: State) -> NestedNode {
        let _buffer = SingletonBuffer::<Option<TypeTerm>>::new( None );

        let data = Arc::new(RwLock::new(ReprTree::new(
            (&ctx, "( Type )")
        )));

        let editor = TypeTermEditor {
            ctx: ctx.clone(),
            state,
            data: data.clone(),
            cur_node: SingletonBuffer::new(node),
            //editor: SingletonBuffer::new(None),
            close_char: SingletonBuffer::new(None),
            spillbuf: Arc::new(RwLock::new(Vec::new()))
        };

        let view = editor.cur_node
            .get_port()
            .map(|node| {
                node.view.clone().unwrap_or(r3vi::view::ViewPort::new().into_outer())
            })
            .to_grid()
            .flatten();
        let cc = editor.cur_node.get().close_char;
        let editor = Arc::new(RwLock::new(editor));

        let mut node = NestedNode::new(ctx, data, depth)
            .set_view(view)
            .set_nav(editor.clone())
            .set_cmd(editor.clone())
            .set_editor(editor.clone());

        editor.write().unwrap().close_char = node.close_char.clone();
        node.spillbuf = editor.read().unwrap().spillbuf.clone();
        
        node
    }

    fn forward_spill(&mut self) {
        eprintln!("forward spill");
        let node = self.cur_node.get();
        let mut buf = node.spillbuf.write().unwrap();
        for n in buf.iter() {
            self.spillbuf.write().unwrap().push(n.clone());
        }
        buf.clear();
    }

    fn send_child_cmd(&mut self, cmd: Arc<RwLock<ReprTree>>) -> TreeNavResult {
        eprintln!("typterm forward cmd");
        let res = self.cur_node.get_mut().send_cmd_obj( cmd );
        self.forward_spill();
        res
    }
    
    fn get_typeterm(&self) -> Option<TypeTerm> {
        match self.state {
            State::Any => None,

            State::AnySymbol => {
                /*
                let x = self.data.descend_ladder(vec![
                    (&ctx, "( FunctionID )").into(),
                    (&ctx, "( Symbol )").into(),
                    (&ctx, "( List Char )").into(),
                ].into_iter());

                let fun_name = /* x...*/ "PosInt";
                let fun_id = self.ctx.read().unwrap().get_typeid( fun_name );

                self.data.add_repr(
                    vec![
                        (&ctx, "( FunctionID )").into(),
                        (&ctx, "( MachineInt )").into()
                    ]
                );
                 */
                Some(TypeTerm::new(TypeID::Fun(0)))
            },
            State::App => {
                Some(TypeTerm::new(TypeID::Fun(0)))
            },

            State::Char => {
                Some(TypeTerm::Char('c'))
            }
            State::Num => {
                Some(TypeTerm::Num(44))
            }
            _ => {None}
        }
    }
}

impl TreeNav for TypeTermEditor {
    fn get_cursor(&self) -> TreeCursor {
        self.cur_node.get().get_cursor()
    }

    fn get_addr_view(&self) -> OuterViewPort<dyn SequenceView<Item = isize>> {
        self.cur_node.get_port().map(|x| x.get_addr_view()).to_sequence().flatten()   
    }

    fn get_mode_view(&self) -> OuterViewPort<dyn SingletonView<Item = ListCursorMode>> {
        self.cur_node.get_port().map(|x| x.get_mode_view()).flatten()
    }

    fn get_cursor_warp(&self) -> TreeCursor {
        self.cur_node.get().get_cursor_warp()
    }

    fn get_max_depth(&self) -> usize {
        self.cur_node.get().get_max_depth()
    }

    fn goby(&mut self, dir: Vector2<isize>) -> TreeNavResult {
        self.cur_node.get_mut().goby(dir)
    }

    fn goto(&mut self, new_cur: TreeCursor) -> TreeNavResult {
        self.cur_node.get_mut().goto(new_cur)
    }
}

impl ObjCommander for TypeTermEditor {
    fn send_cmd_obj(&mut self, co: Arc<RwLock<ReprTree>>) -> TreeNavResult {
        let cmd_obj = co.clone();
        let cmd_obj = cmd_obj.read().unwrap();

        if cmd_obj.get_type().clone() == (&self.ctx, "( Char )").into() {
            if let Some(cmd_view) = cmd_obj.get_view::<dyn SingletonView<Item = char>>() {
                let c = cmd_view.get();

                match &self.state {
                    State::Any => {
                        match c {
                            '<' => {
                                self.set_state( State::App );
                                TreeNavResult::Continue
                            }
                            '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9' => {
                                self.set_state( State::Num );
                                self.send_child_cmd( co );
                                TreeNavResult::Continue
                            }
                            '\'' => {
                                self.set_state( State::Char );
                                TreeNavResult::Continue
                            }
                            '~' => {
                                TreeNavResult::Exit
                            }
                            _ => {
                                self.set_state( State::AnySymbol );
                                self.cur_node.get_mut().goto(TreeCursor::home());
                                self.send_child_cmd( co )
                            }
                        }
                    }
                    State::Char => {
                        match c {
                            '\'' => {
                                self.cur_node.get_mut().goto(TreeCursor::none());
                                TreeNavResult::Exit
                            }
                            _ => {
                                self.send_child_cmd( co )
                            }
                        }
                    }

                    State::Ladder => {
                        eprintln!("have LADDER, send cmd tochild");
                        let res = self.send_child_cmd( co );
                        let cur = self.get_cursor();
                        
                        match res {
                            TreeNavResult::Continue => {
                                if cur.tree_addr.len() == 3 {
                                    match c {
                                        '~' => {
                                            let mut ladder_node = self.cur_node.get().clone();
                                            let mut ladder_edit = ladder_node.get_edit::<ListEditor>().unwrap();

                                            let item = ladder_edit.write().unwrap().get_item().clone();

                                            if let Some(mut it_node) = item {
                                                if it_node.get_type() == (&self.ctx, "( Type )").into() {
                                                    let other_tt = it_node.get_edit::<TypeTermEditor>().unwrap();
                                                    let other = other_tt.read().unwrap().cur_node.get().get_edit::<ListEditor>().unwrap();
                                                    let buf = other.read().unwrap().data.clone();

                                                    ladder_edit.write().unwrap().up();
                                                    ladder_edit.write().unwrap().up();

                                                    ladder_node.send_cmd_obj(
                                                        ListCmd::DeleteNexd.into_repr_tree( &self.ctx )
                                                    );
                                                    //ladder_edit.write().unwrap().delete_nexd();

                                                    let l = buf.len();
                                                    for i in 0..l {
                                                        ladder_edit.write().unwrap().insert( buf.get(i) );
                                                    }
                                                    ladder_node.dn();

                                                    TreeNavResult::Continue
                                                } else {
                                                    TreeNavResult::Continue
                                                }
                                            } else {
                                                TreeNavResult::Continue
                                            }
                                        }
                                        _=> res
                                    }
                                } else {
                                    TreeNavResult::Continue
                                }
                            }
                            res => res,
                        }
                    }

                    State::App => {
                        let res = self.send_child_cmd( co );

                        match res {
                            TreeNavResult::Exit => {
                                match c {
                                    '~' => {
                                        // if item at cursor is Ladder
                                        let app_edit = self.cur_node.get().get_edit::<ListEditor>().expect("editor");
                                        let mut app_edit = app_edit.write().unwrap();
                                        app_edit.delete_nexd();
                                        app_edit.pxev();

                                        if let Some(item_node) = app_edit.get_item() {

                                            let item_typterm = item_node.get_edit::<TypeTermEditor>().expect("typetermedit");
                                            let mut item_typterm = item_typterm.write().unwrap();
                                            match item_typterm.state {
                                                State::Ladder => {
                                                    drop(item_typterm);

                                                    app_edit.dn();
                                                    app_edit.qnexd();
                                                }
                                                _ => {
                                                    eprintln!("create new ladder");
                                                    
                                                    // else create enw ladder
                                                    let mut new_node = Context::make_node( &self.ctx, (&self.ctx, "( List Type )").into(), 0 ).unwrap();
                                                    new_node = new_node.morph( (&self.ctx, "( Type::Ladder )").into() );
                                                    // insert old node and split
                                                    new_node.goto(TreeCursor::home());

                                                    new_node.send_cmd_obj(
                                                        ReprTree::new_leaf(
                                                            (&self.ctx, "( NestedNode )"),
                                                            SingletonBuffer::<NestedNode>::new( item_node ).get_port().into()
                                                        )
                                                    );

                                                    drop(item_typterm);
                                                    *app_edit.get_item_mut().unwrap().write().unwrap() = new_node;
                                                    app_edit.dn();
                                                }
                                            }
                                        }

                                        TreeNavResult::Continue
                                    },
                                    _ => {TreeNavResult::Exit}
                                }
                            },
                            res => res
                        }
                }

                    State::AnySymbol | State::FunSymbol | State::VarSymbol | State::App => {
                        let res = self.send_child_cmd( co );
                        match res {
                            TreeNavResult::Exit => {
                                match c {
                                    '~' => {
                                        eprintln!("typeterm: ~ ");

                                        let old_node = self.cur_node.get().clone();

                                        // create a new NestedNode with TerminaltypeEditor,
                                        // that has same data as current node.
                                        let mut old_edit_node = TypeTermEditor::new_node( self.ctx.clone(), 0 );
                                        let mut old_edit_clone = old_edit_node.get_edit::<TypeTermEditor>().unwrap();
                                        old_edit_clone.write().unwrap().set_state( self.state );
                                        old_edit_clone.write().unwrap().close_char.set( old_node.close_char.get() );
                                        old_edit_clone.write().unwrap().cur_node.set( old_node );

                                        // create new list-edit node for the ladder
                                        let mut new_node = Context::make_node( &self.ctx, (&self.ctx, "( List Type )").into(), 0 ).unwrap();
                                        new_node = new_node.morph( (&self.ctx, "( Type::Ladder )").into() );

                                        eprintln!("insert old node into new node");
                                        
                                        // insert old node and split
                                        new_node.goto(TreeCursor::home());
                                        new_node.send_cmd_obj(
                                            ReprTree::new_leaf(
                                                (&self.ctx, "( NestedNode )"),
                                                SingletonBuffer::new( old_edit_node ).get_port().into()
                                            )
                                        );

                                        new_node.set_addr(0);
                                        new_node.dn();

                                        let res = new_node.send_cmd_obj(
                                            ListCmd::Split.into_repr_tree( &self.ctx )
                                        );

                                        // reconfigure current node to display new_node
                                        self.close_char.set(new_node.close_char.get());
                                        self.cur_node.set(new_node);
                                        self.state = State::Ladder;

                                        TreeNavResult::Continue
                                    }
                                    _ => {
                                        TreeNavResult::Exit
                                    }
                                }
                            }
                            TreeNavResult::Continue => {
                                TreeNavResult::Continue
                            }
                        }
                    }

                    _ => {
                        self.send_child_cmd( co )
                    }
                }
            } else {
                TreeNavResult::Exit
            }
        } else {
            match &self.state {
                State::Any => {
                    self.set_state( State::AnySymbol );
                    self.cur_node.get_mut().goto(TreeCursor::home());
                }
                _ => {
                }
            }

            self.send_child_cmd( co )
        }
    }
}

