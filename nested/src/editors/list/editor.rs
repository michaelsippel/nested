use {
    r3vi::{
        view::{ChannelSender, ChannelReceiver, port::UpdateTask, OuterViewPort, singleton::*, sequence::*},
        buffer::{singleton::*, vec::*}
    },
    crate::{
        type_system::{Context, TypeTerm, ReprTree},
        editors::list::{ListCursor, ListCursorMode, ListCmd},
        tree::{NestedNode, TreeNav, TreeCursor},
        diagnostics::Diagnostics,
        commander::ObjCommander
    },
    std::sync::{Arc, RwLock, Mutex},
    std::ops::Deref
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub struct ListEditor {
    pub(super) cursor: SingletonBuffer<ListCursor>,

    // todo: (?) remove RwLock<..> around NestedNode ??
    pub data: VecBuffer< Arc<RwLock<NestedNode>> >,

    pub spillbuf: Arc<RwLock<Vec<Arc<RwLock<NestedNode>>>>>,
    
    pub(super) addr_port: OuterViewPort<dyn SequenceView<Item = isize>>,
    pub(super) mode_port: OuterViewPort<dyn SingletonView<Item = ListCursorMode>>,

    pub(crate) ctx: Arc<RwLock<Context>>,

    /// item type
    pub(super) typ: TypeTerm,
}

impl ListEditor {
    pub fn new(
        ctx: Arc<RwLock<Context>>,
        typ: TypeTerm,
    ) -> Self {
        let cursor = SingletonBuffer::new(ListCursor::default());
        let data : VecBuffer<Arc<RwLock<NestedNode>>> = VecBuffer::new();

        ListEditor {
            mode_port: cursor
                .get_port()
                .map({
                    let data = data.clone();
                    move |c| {
                        let ip = SingletonBuffer::new(c.mode).get_port();
                        match c.mode {
                            ListCursorMode::Insert => ip,
                            ListCursorMode::Select => {
                                if let Some(idx) = c.idx {
                                    if idx > 0 && idx < data.len() as isize {
                                        data.get(idx as usize).read().unwrap().get_mode_view()
                                    } else {
                                        eprintln!("ListEditor::mode_port invalid cursor idx");
                                        ip
                                    }
                                } else {
                                    ip
                                }
                            }
                        }
                    }
                })
                .flatten(),

            addr_port: VecBuffer::<OuterViewPort<dyn SequenceView<Item = isize>>>::with_data(
                vec![
                    cursor.get_port()
                        .to_sequence()
                        .filter_map(|cur| cur.idx),
                    cursor.get_port()
                        .map({
                            let data = data.clone();
                            move |cur| {
                                if cur.mode == ListCursorMode::Select {
                                    if let Some(idx) = cur.idx {
                                        if idx >= 0 && idx < data.len() as isize {
                                            return data.get(idx as usize).read().unwrap().get_addr_view();
                                        }
                                    }
                                }
                                OuterViewPort::default()
                            }
                        })
                        .to_sequence()
                        .flatten()                
                ])
                .get_port()
                .to_sequence()
                .flatten(),
            cursor,
            data,
            spillbuf: Arc::new(RwLock::new(Vec::new())),
            ctx,
            typ
        }
    }

    pub fn into_node(self, depth: usize) -> NestedNode {
        let data = self.get_data();
        let ctx = self.ctx.clone();
        let editor = Arc::new(RwLock::new(self));

        let e = editor.read().unwrap();

        let mut node = NestedNode::new(ctx, data, depth)
            .set_editor(editor.clone())
            .set_nav(editor.clone())
            .set_cmd(editor.clone())
            .set_diag(e
                      .get_data_port()
                      .enumerate()
                      .map(
                          |(idx, item_editor)| {
                              let idx = *idx;
                              item_editor
                                  .get_msg_port()
                                  .map(
                                      move |msg| {
                                          let mut msg = msg.clone();
                                          msg.addr.insert(0, idx);
                                          msg
                                      }
                                  )
                          }
                      )
                      .flatten()
            );

        node.spillbuf = e.spillbuf.clone();
        node
    }

    pub fn get_item_type(&self) -> TypeTerm {
        self.typ.clone()
    }

    pub fn get_seq_type(&self) -> TypeTerm {
        TypeTerm::App(vec![
            TypeTerm::TypeID(self.ctx.read().unwrap().get_typeid("List").unwrap()),
            self.get_item_type().into()
        ])
    }

    pub fn get_cursor_port(&self) -> OuterViewPort<dyn SingletonView<Item = ListCursor>> {
        self.cursor.get_port()
    }

    pub fn get_data_port(&self) -> OuterViewPort<dyn SequenceView<Item = NestedNode>> {
        self.data.get_port().to_sequence().map(
            |x| x.read().unwrap().clone()
        )
    }

    pub fn get_data(&self) -> Arc<RwLock<ReprTree>> {
        let data_view = self.get_data_port();
        ReprTree::new_leaf(
            self.get_seq_type(),
            data_view.into()
        )
    }

    pub fn get_item(&self) -> Option<NestedNode> {
        if let Some(idx) = self.cursor.get().idx {
            let idx = crate::utils::modulo(idx as isize, self.data.len() as isize) as usize;
            if idx < self.data.len() {
                Some(self.data.get(idx).read().unwrap().clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_item_mut(&mut self) -> Option<MutableVecAccess<Arc<RwLock<NestedNode>>>> {
        if let Some(idx) = self.cursor.get().idx {
            let idx = crate::utils::modulo(idx as isize, self.data.len() as isize) as usize;
            if idx < self.data.len() {
                Some(self.data.get_mut(idx))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn is_listlist(&self) -> bool {
        self.ctx.read().unwrap().is_list_type(&self.typ)
    }

    /// delete all items
    pub fn clear(&mut self) {
        eprintln!("list editor: clear");
        let mut b = self.spillbuf.write().unwrap();
        for i in 0..self.data.len() {
            b.push( self.data.get(i) );
        }
        
        self.data.clear();
        self.cursor.set(ListCursor::home());
    }

    /// delete item before the cursor
    pub fn delete_pxev(&mut self) {
        let mut cur = self.cursor.get();
        if let Some(idx) = cur.idx {
            if idx > 0 && idx <= self.data.len() as isize {
                cur.idx = Some(idx as isize - 1);
                self.cursor.set(cur);
                self.data.remove(idx as usize - 1);
            }
        }
    }

    /// delete item after the cursor
    pub fn delete_nexd(&mut self) {
        if let Some(idx) = self.cursor.get().idx {
            if idx < self.data.len() as isize {
                self.data.remove(idx as usize);
            }
        }
    }

    /// insert a new element
    pub fn insert(&mut self, item: Arc<RwLock<NestedNode>>) {
        let mut cur = self.cursor.get();
        if let Some(idx) = cur.idx {
            match cur.mode {
                ListCursorMode::Insert => {
                    self.data.insert(idx as usize, item.clone());
                    if self.is_listlist() {
                        cur.mode = ListCursorMode::Select;
                    } else {
                        cur.idx = Some(idx + 1);               
                    }
                }

                ListCursorMode::Select => {
                    self.data.insert(1 + idx as usize, item.clone());                    
                    if self.is_listlist() {
                        cur.idx = Some(idx + 1);
                    }
                }
            }

            self.cursor.set(cur);
        } else {
            //eprintln!("insert: no cursor");
        }
    }

    /// split the list off at the current cursor position and return the second half
    pub fn split(&mut self) {        
        let cur = self.cursor.get();
        if let Some(idx) = cur.idx {
            let idx = idx as usize;
            for _ in idx .. self.data.len() {
                self.spillbuf.write().unwrap().push(
                    self.data.get(idx)
                );
                self.data.remove(idx);
            }

            /* TODO
             */
            /*
            if self.is_listlist() {
                if idx > 0 && idx < self.data.len()+1 {

                    let prev_idx = idx - 1; // get last element before cursor (we are in insert mode)
                    let prev_node = self.data.get(prev_idx);
                    let prev_node = prev_node.read().unwrap();

                    if let Some(prev_editor) = prev_node.editor.get() {
                        let prev_editor = prev_editor.downcast::<RwLock<ListEditor>>().unwrap();
                        let prev_editor = prev_editor.write().unwrap();
                        prev_editor.get_data_port().0.update();

                        if prev_editor.get_data_port().get_view().unwrap().iter()
                            .filter_map(|x| x.get_data_view::<dyn SingletonView<Item = Option<char>>>(vec![].into_iter())?.get()).count() == 0
                        {
                            drop(prev_editor);
                            self.data.remove(prev_idx);
                        }
                    }
                }
        }
            */
        }
    }

    pub fn listlist_split(&mut self) {
        let cur = self.get_cursor();
        eprintln!("listlist_split(): cur = {:?}", cur);
        if let Some(mut item) = self.get_item().clone() {
            eprintln!("listlist_split(): split child item");
            item.send_cmd_obj(ListCmd::Split.into_repr_tree(&self.ctx));
            eprintln!("listlist_split(): done child split");

            if cur.tree_addr.len() < 3 {
                item.goto(TreeCursor::none());
                let mut tail_node = Context::make_node(&self.ctx, self.typ.clone(), 0).unwrap();
                //tail_node = tail_node.morph(  );
                tail_node.goto(TreeCursor::home());

                let mut b = item.spillbuf.write().unwrap();
                for node in b.iter() {
                    tail_node
                        .send_cmd_obj(
                            ReprTree::new_leaf(
                                (&self.ctx, "( NestedNode )"),
                                SingletonBuffer::<NestedNode>::new(
                                    node.read().unwrap().clone()
                                ).get_port().into()
                            )
                        );
                }
                b.clear();
                drop(b);
                drop(item);

                self.set_leaf_mode(ListCursorMode::Insert);
                self.nexd();

                tail_node.goto(TreeCursor::home());
                if cur.tree_addr.len() > 2 {
                    tail_node.dn();
                }

                eprintln!("insert tail node");
                self.insert(
                    Arc::new(RwLock::new(tail_node))
                );
            } else {
                self.up();
                self.listlist_split();
                self.dn();
                eprintln!("tree depth >= 3");
            }
        }
    }

    pub fn listlist_join_pxev(&mut self, idx: isize) {
        {
            let cur_editor = self.data.get(idx as usize);
            let pxv_editor = self.data.get(idx as usize-1);
            let mut cur_editor = cur_editor.write().unwrap();
            let mut pxv_editor = pxv_editor.write().unwrap();

            let oc0 = cur_editor.get_cursor();

            // tell cur_editor move all its elements into its spill-buffer
            cur_editor.goto(TreeCursor::none());
            cur_editor.send_cmd_obj(
                ListCmd::Clear.into_repr_tree( &self.ctx )
            );
            
            pxv_editor.goto(TreeCursor {
                tree_addr: vec![-1],
                leaf_mode: ListCursorMode::Insert
            });

            let old_cur = pxv_editor.get_cursor();

            let data = cur_editor.spillbuf.read().unwrap();
            for x in data.iter() {
                pxv_editor.send_cmd_obj(
                    ReprTree::new_leaf(
                        (&self.ctx, "( NestedNode )"),
                        SingletonBuffer::<NestedNode>::new(
                            x.read().unwrap().clone()
                        ).get_port().into()
                    )
                );
            }

            if oc0.tree_addr.len() > 1 {
                pxv_editor.goto(TreeCursor {
                    tree_addr: vec![ old_cur.tree_addr[0], 0 ],
                    leaf_mode: ListCursorMode::Insert                
                });
                pxv_editor.send_cmd_obj(ListCmd::DeletePxev.into_repr_tree( &self.ctx ));
            } else {
                pxv_editor.goto(TreeCursor {
                    tree_addr: vec![ old_cur.tree_addr[0] ],
                    leaf_mode: ListCursorMode::Insert                
                });
            }
        }

        self.cursor.set(ListCursor {
            idx: Some(idx as isize - 1),
            mode: ListCursorMode::Select
        });

        // remove cur_editor from top list, its elements are now in pxv_editor
        self.data.remove(idx as usize);
    }

    pub fn listlist_join_nexd(&mut self, idx: usize) {
        eprintln!("listilst_join_nexd");
        {
            let cur_editor = self.data.get(idx);
            let nxd_editor = self.data.get(idx + 1);
            let mut cur_editor = cur_editor.write().unwrap();
            let mut nxd_editor = nxd_editor.write().unwrap();

            let oc0 = cur_editor.get_cursor();

            // tell next_editor move all its elements into its spill-buffer
            nxd_editor.goto(TreeCursor::none());
            nxd_editor.send_cmd_obj(
                ListCmd::Clear.into_repr_tree( &self.ctx )
            );

            let old_cur = cur_editor.get_cursor();
            cur_editor.goto(TreeCursor {
                tree_addr: vec![-1],
                leaf_mode: ListCursorMode::Insert
            });
 
            let data = nxd_editor.spillbuf.read().unwrap();
            eprintln!("spillbuf of next : {} elements", data.len());
            for x in data.iter() {
                cur_editor.send_cmd_obj(
                    ReprTree::new_leaf(
                        (&self.ctx, "( NestedNode )"),
                        SingletonBuffer::<NestedNode>::new(
                            x.read().unwrap().clone()
                        ).get_port().into()
                    )
                );
            }

            if oc0.tree_addr.len() > 1 {
                cur_editor.goto(TreeCursor {
                    tree_addr: vec![ old_cur.tree_addr[0], -1 ],
                    leaf_mode: ListCursorMode::Insert                
                });
                cur_editor.send_cmd_obj(ListCmd::DeleteNexd.into_repr_tree( &self.ctx ));
            } else {
                cur_editor.goto(TreeCursor {
                    tree_addr: vec![ old_cur.tree_addr[0] ],
                    leaf_mode: ListCursorMode::Insert                
                });
            }
        }

        // remove next_editor from top list, its elements are now in cur_editor
        self.data.remove(idx+1);
    }
}

/*
use crate::{
    type_system::TypeLadder,
    tree::{TreeType, TreeAddr}
};

impl TreeType for ListEditor {
    fn get_type(&self, addr: &TreeAddr) -> TypeLadder {
        let idx = crate::utils::modulo::modulo(addr.0[0] as isize, self.data.len() as isize) as usize;

        let mut addr = addr.clone();
        
        if self.data.len() > 0 {
            addr.0.remove(0);
            self.data.get(idx).get_type(addr)
        } else {
            vec![]
        }
    }
}
 */


