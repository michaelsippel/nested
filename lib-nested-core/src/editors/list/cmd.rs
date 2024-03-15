use {
    r3vi::{
        view::{singleton::*}
    },
    crate::{
        editors::{list::{ListEditor, ListCursor, ListCursorMode}, ObjCommander},
        repr_tree::{Context, ReprTree},
        edit_tree::{EditTree, TreeNav, TreeNavResult, TreeCursor},
    },
    std::sync::{Arc, RwLock}
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ListCmd {
    DeletePxev,
    DeleteNexd,
    JoinNexd,
    JoinPxev,
    Split,
    Clear,
    Close,
}

impl ListCmd {
    // note: cant use Into becaue of ctx (maybe global typedict?)
    pub fn into_repr_tree(self, ctx: &Arc<RwLock<Context>>) -> Arc<RwLock<ReprTree>> {
        ReprTree::from_singleton_buffer(
            Context::parse(ctx, "ListCmd"),            
            r3vi::buffer::singleton::SingletonBuffer::new(self)
        )
    }
}

impl ObjCommander for ListEditor {
    fn send_cmd_obj(&mut self, cmd_obj: Arc<RwLock<ReprTree>>) -> TreeNavResult {
        let cmd_repr = cmd_obj.read().unwrap();

        if let Some(view) = cmd_repr.get_view::<dyn SingletonView<Item = EditTree>>() {
            let node = view.get();
            let cur = self.cursor.get();

            if let Some(idx) = cur.idx {
                match cur.mode {
                    ListCursorMode::Select => {
                        *self.data.get_mut(idx as usize) = Arc::new(RwLock::new(node));
                        TreeNavResult::Exit
                    }
                    ListCursorMode::Insert => {
                        self.insert(Arc::new(RwLock::new(node)));
                        self.cursor.set(ListCursor{ idx: Some(idx+1),  mode: ListCursorMode::Insert });
                        TreeNavResult::Continue
                    }
                }
            } else {
                TreeNavResult::Exit
            }
        }

        else if let Some(cmd) = cmd_repr.get_view::<dyn SingletonView<Item = ListCmd>>() {

            let cur = self.cursor.get();
            drop(cmd_repr);

            match cmd.get() {
                ListCmd::Clear => {
                    self.clear();
                    TreeNavResult::Continue
                }
                _ => {
                    if let Some(idx) = cur.idx {
                        match cur.mode {
                            ListCursorMode::Select => {                        
                                if let Some(mut item) = self.get_item().clone() {
                                    let item_cur = item.get_cursor();
                                    
                                    match cmd.get() {
                                        ListCmd::DeletePxev => {
                                            if idx > 0
                                                && item_cur.tree_addr.iter().fold(
                                                    true,
                                                    |is_zero, x| is_zero && (*x == 0)
                                                )
                                            {
                                                self.listlist_join_pxev(idx);
                                                TreeNavResult::Continue
                                            } else {
                                                item.send_cmd_obj(cmd_obj)
                                            }
                                        }

                                        ListCmd::DeleteNexd => {
                                            let item_cur = item.get_cursor_warp();
                                            let next_idx = idx as usize + 1;

                                            if next_idx < self.data.len()
                                                && item_cur.tree_addr.iter().fold(
                                                    true,
                                                    |is_end, x| is_end && (*x == -1)
                                                )
                                            {
                                                self.listlist_join_nexd(idx as usize);
                                                TreeNavResult::Continue
                                            } else {
                                                item.send_cmd_obj(cmd_obj)
                                            }
                                        }

                                        ListCmd::Split => {
                                            self.listlist_split();
                                            TreeNavResult::Continue
                                        }

                                        _ => {
                                            item.send_cmd_obj(cmd_obj);
                                            TreeNavResult::Continue
                                        }
                                    }
                                } else {
                                    TreeNavResult::Exit
                                }
                            },

                            ListCursorMode::Insert => {
                                match cmd.get() {
                                    ListCmd::DeletePxev => {
                                        self.delete_pxev();
                                        TreeNavResult::Continue
                                    }
                                    ListCmd::DeleteNexd => {
                                        self.delete_nexd();
                                        TreeNavResult::Continue
                                    }
                                    ListCmd::Split => {
                                        self.split();
                                        TreeNavResult::Exit
                                    }
                                    ListCmd::Close => {
                                        self.goto(TreeCursor::none());
                                        TreeNavResult::Exit
                                    }
                                    _ =>{
                                        TreeNavResult::Continue
                                    }
                                }
                            }
                        }
                    } else {
                        TreeNavResult::Exit
                    }
                }
            }

        } else {
            if let Some(cur_item) = self.get_item_mut() {
                drop(cmd_repr);
                cur_item.write().unwrap().send_cmd_obj(cmd_obj);
                TreeNavResult::Continue
            } else {
                TreeNavResult::Exit
            }
        }
    }
}

