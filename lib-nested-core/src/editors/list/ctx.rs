use {
    r3vi::{
        view::{
            ViewPort, port::UpdateTask,
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
/*
    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>"),
        dst_type: Context::parse(&ctx, "<List Char~EditTree>")
    };
    ctx.write().unwrap().morphisms.add_morphism(mt, {
        let ctx = ctx.clone();
        move |src_rt, σ| {
            let list_port = src_rt.descend(Context::parse(&ctx, "<List Char>")).expect("descend").get_port::<dyn ListView<char>>().clone();
            if let Some(list_port) = list_port {

                // for each char, create EditTree
                let edit_tree_list = 
                        list_port
                        .map({
                            let ctx = ctx.clone();
                            move |c| {
                                let item_rt = ReprTree::from_char(&ctx, *c);

                                ctx.read().unwrap().setup_edittree(
                                    item_rt.clone(),
                                    SingletonBuffer::new(0).get_port()
                                );

                                let et = item_rt
                                    .descend(Context::parse(&ctx, "Char ~ EditTree")).expect("cant descend repr tree")
                                    .get_port::< dyn SingletonView<Item = EditTree> >().expect("cant get view port (EditTree)")
                                    .get_view().unwrap()
                                    .get();
                                Arc::new(RwLock::new(et))
                            }
                        });

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "<List Char>~<List EditTree>"),
                    edit_tree_list
                );
            } else {
                eprintln!("morphism missing view port");
            }
        }
    });
*/
    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Item>~<List EditTree>~<Vec EditTree>"),
        dst_type: Context::parse(&ctx, "<List Item>~EditTree")
    };

    ctx.write().unwrap().morphisms.add_morphism(mt, {
        let ctx = ctx.clone();
        move |src_rt, σ| {
            let item_id = laddertypes::TypeID::Var( ctx.read().unwrap().get_var_typeid("Item").unwrap() );
            if let Some( item_type ) = σ.get( &item_id ) {
                let mut item_vec_rt = src_rt
                    .descend(
                        Context::parse(&ctx, "<List Item~EditTree>~<Vec EditTree>")
                            .apply_substitution(&|id| σ.get(id).cloned()).clone()
                    )
                    .expect("cant descend src repr");

                let item_vec_buffer = item_vec_rt.vec_buffer::< Arc<RwLock<EditTree>> >();

                let mut list_editor = ListEditor::with_data(ctx.clone(), item_type.clone(), item_vec_buffer);
                let edittree_list = list_editor.into_node(
                    SingletonBuffer::<usize>::new(0).get_port()
                );
                src_rt.insert_leaf(
                    Context::parse(&ctx, "<List Item> ~ EditTree")
                        .apply_substitution(&|id| σ.get(id).cloned()).clone(),
                    ReprLeaf::from_singleton_buffer(
                        SingletonBuffer::new(Arc::new(RwLock::new(edittree_list)))
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
                        .descend(Context::parse(&ctx, "<List Char>~EditTree")).unwrap()
                        .singleton_buffer::<Arc<RwLock<EditTree>>>();

                let list_edit = edittree.get().read().unwrap().get_edit::< ListEditor >().unwrap();
                let edittree_items = list_edit.read().unwrap().data.get_port().to_list();

                src_rt.insert_leaf(
                    Context::parse(&ctx, "<List Char>"),
                    ReprLeaf::from_view(
                    edittree_items
                        .map(|edittree_char|
                            edittree_char
                                .read().unwrap()
                                .get_edit::<CharEditor>().unwrap()
                                .read().unwrap()
                                .get()
                        ))
                );
            }
        }
    );


    /* todo : unify the following two morphims with generic item parameter ?
     */
    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>"),
        dst_type: Context::parse(&ctx, "<List Char>~<Vec Char>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                src_rt
                    .attach_leaf_to(
                        Context::parse(&ctx, "<List Char>~<Vec Char>"),
                        src_rt
                            .descend(Context::parse(&ctx, "<List Char>"))
                            .expect("descend")
                            .view_list::<char>()
                    );
            }
        }
    );

    let mt = crate::repr_tree::MorphismType {
        src_type: Context::parse(&ctx, "<List Char>~<Vec Char>"),
        dst_type: Context::parse(&ctx, "<List Char>")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_port = src_rt.descend(Context::parse(&ctx, "<List Char>~<Vec Char>")).expect("descend")
                    .get_port::<RwLock<Vec<char>>>().unwrap();
                src_rt.attach_leaf_to( Context::parse(&ctx, "<List Char>"), src_port.to_list() );              
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
                let p = 
                    src_rt
                        .descend(Context::parse(&ctx, "<List EditTree>")).expect("descend")
                        .get_port::<dyn ListView< Arc<RwLock<EditTree>> >>().unwrap();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "<List EditTree> ~ <Vec EditTree>"),
                    p
                );
            }
        }
    );
}

