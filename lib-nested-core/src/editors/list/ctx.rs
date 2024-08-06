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
        repr_tree::{Context, ReprTree, ReprLeaf, ReprTreeExt, GenericReprTreeMorphism},
        edit_tree::{EditTree},
        editors::{
            char::{CharEditor},
            list::{ListEditor}
        }
    },
    std::sync::{Arc, RwLock}
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub fn init_ctx(ctx: Arc<RwLock<Context>>) {
    ctx.write().unwrap().add_varname("Item");

    let list_morph_editsetup1 = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "<List Item>~<List EditTree>~<Vec EditTree>"),
        Context::parse(&ctx, "<List Item>~EditTree"),
        {
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
        }
    );


    let list_morph_editsetup2 = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "<List Char>~EditTree"),
        Context::parse(&ctx, "<List Char>"),
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
                        )
                    )
                );
            }
            
        }
    );

    let list_morph_to_vec_char = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "<List Char>"),
        Context::parse(&ctx, "<List Char>~<Vec Char>"),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "<Vec Char>"),
                    src_rt.view_list::<char>()
                );
            }
        }
    );

    let list_morph_from_vec_char = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "<List Char>~<Vec Char>"),
        Context::parse(&ctx, "<List Char>"),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_port = src_rt.descend(Context::parse(&ctx, "<List Char>~<Vec Char>")).expect("descend")
                    .get_port::<RwLock<Vec<char>>>().unwrap();

                src_rt.attach_leaf_to( Context::parse(&ctx, "<List Char>"), src_port.to_list() );
            }
        }
    );


    let list_morph_to_vec_edittree = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "<List EditTree>"),
        Context::parse(&ctx, "<List EditTree> ~ <Vec EditTree>"),

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

    ctx.write().unwrap().morphisms.add_morphism( list_morph_editsetup1 );
    ctx.write().unwrap().morphisms.add_morphism( list_morph_editsetup2 );
    ctx.write().unwrap().morphisms.add_morphism( list_morph_from_vec_char );
    ctx.write().unwrap().morphisms.add_morphism( list_morph_to_vec_char );
    ctx.write().unwrap().morphisms.add_morphism( list_morph_to_vec_edittree );
}

