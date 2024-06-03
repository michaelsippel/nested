use {
    r3vi::{
        view::{
            ViewPort,
            OuterViewPort, Observer,
            singleton::*,
            list::*
        },
        buffer::{singleton::*, vec::*}
    },
    laddertypes::{TypeTerm},
    crate::{
        repr_tree::{Context, ReprTree, ReprLeaf, ReprTreeExt},
        edit_tree::{EditTree},
        editors::{
            char::{CharEditor},
            list::{ListEditor}//, PTYListController, PTYListStyle}
        }
    },
    std::sync::{Arc, RwLock}
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub fn init_ctx(ctx: Arc<RwLock<Context>>) {
    ctx.write().unwrap().add_varname("Item");

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>"),
        dst_type: Context::parse(&ctx, "<List Char~EditTree>")
    };
    ctx.write().unwrap().morphisms.add_morphism(mt, {
        let ctx = ctx.clone();
        move |src_rt, σ| {
            let list_port = src_rt.read().unwrap().get_port::<dyn ListView<char>>().clone();
            if let Some(list_port) = list_port {
                let edit_tree_list = 
                        list_port
                        // for each char, create and EditTree
                        .map({
                            let ctx = ctx.clone();
                            move |c| {
                                let item_rt = ReprTree::from_char(&ctx, *c);
                                ctx.read().unwrap().setup_edittree(
                                    item_rt.clone(),
                                    SingletonBuffer::new(0).get_port()
                                );
                                let et = item_rt
                                    .descend(Context::parse(&ctx, "EditTree")).unwrap()
                                    .read().unwrap()
                                    .get_port::< dyn SingletonView<Item = EditTree> >()
                                    .expect("cant get view port (EditTree)")
                                    .get_view().unwrap()
                                    .get();
                                Arc::new(RwLock::new(et))
                            }
                        });

                src_rt.write().unwrap().insert_leaf(
                    Context::parse(&ctx, "<List EditTree>").get_lnf_vec().into_iter(),
                    ReprLeaf::from_view( edit_tree_list )
                );
            } else {
                eprintln!("morphism missing view port");
            }
        }
    });

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Item>~<List EditTree>~<Vec EditTree>"),
        dst_type: Context::parse(&ctx, "<List Item>~EditTree")
    };
    ctx.write().unwrap().morphisms.add_morphism(mt, {
        let ctx = ctx.clone();
        move |src_rt, σ| {
            let item_id = laddertypes::TypeID::Var( ctx.read().unwrap().get_var_typeid("Item").unwrap() );
            if let Some( item_type ) = σ.get( &item_id ) {
                /*
                let mut item_vec_buffer = VecBuffer::new();

                eprintln!("try attach to data port");
                if let Some( list_port ) =
                    src_rt
                        .descend(Context::parse(&ctx, "<List EditTree>")).expect("")
                        .read().unwrap()
                        .get_port::< dyn ListView< Arc<RwLock<EditTree>> > >()
                {
                    eprintln!("get list<edittree> port");
                    item_vec_buffer.attach_to( list_port );
                }*/

                let mut item_vec_rt = src_rt
                    .descend(Context::parse(&ctx, "<List EditTree>~<Vec EditTree>"))
                    .expect("cant descend src repr");

                let item_vec_buffer = item_vec_rt
                    .write().unwrap()
                    .vec_buffer::< Arc<RwLock<EditTree>> >().expect("cant get vec buffer");

                // eprintln!("create ListEditor");
                let mut list_editor = ListEditor::with_data(ctx.clone(), item_type.clone(), item_vec_buffer);

                let edittree_list = list_editor.into_node(
                    SingletonBuffer::<usize>::new(0).get_port()
                );

               // eprintln!("make edittree");
                src_rt.write().unwrap().insert_branch(
                    ReprTree::from_singleton_buffer(
                        Context::parse(&ctx, "EditTree"),
                        SingletonBuffer::new(edittree_list)
                    )
                );
            } else {
                eprintln!("no item type");
            }
        }
    });

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>~EditTree"),
        dst_type: Context::parse(&ctx, "<List Char>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let edittree =
                    src_rt
                        .descend(Context::parse(&ctx, "EditTree")).unwrap()
                        .singleton_buffer::<EditTree>();

                let list_edit = edittree.get().get_edit::< ListEditor >().unwrap();
                let edittree_items = list_edit.read().unwrap().data.get_port().to_list();
                src_rt.write().unwrap().insert_leaf(
                    vec![].into_iter(),
                    ReprLeaf::from_view(
                        edittree_items
                            .map(
                                |edittree_char|
                                    edittree_char
                                    .read().unwrap()
                                    .get_edit::<CharEditor>().unwrap()
                                    .read().unwrap()
                                    .get()
                            )
                    )
                );
            }
        }
    );

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List <Digit Radix>>~EditTree"),
        dst_type: Context::parse(&ctx, "<List <Digit Radix>~Char>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let edittree =
                    src_rt
                        .descend(Context::parse(&ctx, "EditTree")).unwrap()
                        .singleton_buffer::<EditTree>();

                let list_edit = edittree.get().get_edit::< ListEditor >().unwrap();
                let edittree_items = list_edit.read().unwrap().data.get_port().to_list();
                src_rt.write().unwrap().insert_leaf(
                    vec![ Context::parse(&ctx, "<List Char>") ].into_iter(),
                    ReprLeaf::from_view(
                        edittree_items
                            .map(
                                |edittree_char|
                                    edittree_char
                                    .read().unwrap()
                                    .get_edit::<crate::editors::digit::editor::DigitEditor>().unwrap()
                                    .read().unwrap()
                                    .get_char()
                            )
                    )
                );
            }
        }
    );


    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>"),
        dst_type: Context::parse(&ctx, "<List Char>~<Vec Char>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let buf = VecBuffer::<char>::new();
                let mut leaf = ReprLeaf::from_vec_buffer(buf);
                leaf.attach_to(
                    src_rt.read().unwrap()
                        .get_port::<dyn ListView<char>>()
                        .unwrap()
                );
                src_rt.write().unwrap().insert_leaf(
                    vec![ Context::parse(&ctx, "<Vec Char>") ].into_iter(),
                    leaf
                );
            }
        }
    );

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List EditTree>"),
        dst_type: Context::parse(&ctx, "<List EditTree>~<Vec EditTree>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let buf = VecBuffer::<Arc<RwLock<EditTree>>>::new();
                let mut leaf = ReprLeaf::from_vec_buffer(buf);
                leaf.attach_to(
                    src_rt.read().unwrap()
                        .get_port::<dyn ListView< Arc<RwLock<EditTree>> >>()
                        .unwrap()
                );
                src_rt.write().unwrap().insert_leaf(
                    vec![ Context::parse(&ctx, "<Vec EditTree>") ].into_iter(),
                    leaf
                );
            }
        }
    );
}

