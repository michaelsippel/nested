use {
    laddertypes::{TypeTerm, TypeID},
    r3vi::view::{AnyOuterViewPort, port::UpdateTask},
    crate::{
        repr_tree::{ReprTree, ReprTreeExt, ReprLeaf},
    },
    std::{
        sync::{Arc, RwLock},
        collections::HashMap
    }
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct MorphismType {
    pub src_type: TypeTerm,
    pub dst_type: TypeTerm,
}

#[derive(Clone)]
pub struct GenericReprTreeMorphism {
    morph_type: MorphismType,
    setup_projection: Arc<
        dyn Fn( &mut Arc<RwLock<ReprTree>>, &HashMap<TypeID, TypeTerm> )
//            -> Result< ReprLeaf, () >
        + Send + Sync
    >
}

#[derive(Clone)]
pub struct MorphismBase {
    morphisms: Vec< GenericReprTreeMorphism >
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

impl MorphismBase {
    pub fn new() -> Self {
        MorphismBase {
            morphisms: Vec::new()
        }
    }

    pub fn add_morphism(
        &mut self,
        morph_type: MorphismType,
        setup_projection:
            impl Fn( &mut Arc<RwLock<ReprTree>>, &HashMap<TypeID, TypeTerm> )
//                -> Result< ReprLeaf, () /* TODO: error */ >
            + Send + Sync + 'static
    ) {
        self.morphisms.push(
            GenericReprTreeMorphism {
                morph_type: MorphismType {
                    src_type: morph_type.src_type.normalize(),
                    dst_type: morph_type.dst_type.normalize()
                },
                setup_projection: Arc::new(setup_projection)
            }
        );
    }

    pub fn find_morphism(
        &self,
        src_type: &TypeTerm,
        dst_type: &TypeTerm
    ) -> Option<(GenericReprTreeMorphism, HashMap<TypeID, TypeTerm>)> {

        // try list-map morphism
        if let Ok(σ) = laddertypes::UnificationProblem::new(vec![
            (src_type.clone().param_normalize(), TypeTerm::App(vec![ TypeTerm::TypeID(TypeID::Fun(10)), TypeTerm::TypeID(TypeID::Var(100)) ])),
            (dst_type.clone().param_normalize(), TypeTerm::App(vec![ TypeTerm::TypeID(TypeID::Fun(10)), TypeTerm::TypeID(TypeID::Var(101)) ])),
        ]).solve() {
            let src_item_type = σ.get(&TypeID::Var(100)).unwrap().clone();
            let dst_item_type = σ.get(&TypeID::Var(101)).unwrap().clone();
/*
            eprintln!("Got a List- Type, check if we find a list-map morphism");
            eprintln!("src-item-type : {:?}", src_item_type);
            eprintln!("dst-item-type : {:?}", dst_item_type);
*/
            let src_item_type_lnf = src_item_type.clone().get_lnf_vec();
            let dst_item_type_lnf = dst_item_type.clone().get_lnf_vec();

            /* if src_item_type ~== "Char",
                  dst_item_type ~== "machine.UInt64"
             */
            if src_item_type_lnf.last() == Some(&TypeTerm::TypeID(TypeID::Fun(0))) &&
                dst_item_type_lnf.last() == Some(&TypeTerm::TypeID(TypeID::Fun(4)))
            {
                if let Some((m, σ)) = self.find_list_map_morphism::< char, u64 >( src_item_type, dst_item_type ) {
                    return Some((m, σ));
                }
            }

            /* if src_item_type ~== "Char"
                  dst_item_type ~== "EditTree"
             */
            else if src_item_type_lnf.last() == Some(&TypeTerm::TypeID(TypeID::Fun(0))) &&
                dst_item_type_lnf.last() == Some(&TypeTerm::TypeID(TypeID::Fun(1)))
            {
                if let Some((m, σ)) = self.find_list_map_morphism::< char, Arc<RwLock<crate::edit_tree::EditTree>> >( src_item_type, dst_item_type ) {
                    return Some((m, σ));
                }
            }
         }

        // otherwise
        for m in self.morphisms.iter() {
            let unification_problem = laddertypes::UnificationProblem::new(
                vec![
                    ( src_type.clone().normalize(), m.morph_type.src_type.clone() ),
                    ( dst_type.clone().normalize(), m.morph_type.dst_type.clone() )
                ]
            );

            let unification_result = unification_problem.solve();
            if let Ok(σ) = unification_result {
                return Some((m.clone(), σ));
            }
        }

        None
    }


    pub fn find_morphism_ladder(
        &self,
        src_type: &TypeTerm,
        dst_type: &TypeTerm,
    ) -> Option<(
        GenericReprTreeMorphism,
        TypeTerm,
        HashMap<TypeID, TypeTerm>
    )> {
        let mut src_lnf = src_type.clone().get_lnf_vec();
        let mut dst_lnf = dst_type.clone().get_lnf_vec();
        let mut halo = vec![];

        while src_lnf.len() > 0 && dst_lnf.len() > 0 {
            if let Some((m, σ)) = self.find_morphism( &TypeTerm::Ladder(src_lnf.clone()), &TypeTerm::Ladder(dst_lnf.clone()) ) {
                return Some((m, TypeTerm::Ladder(halo), σ));
            } else {
                if src_lnf[0] == dst_lnf[0] {
                    src_lnf.remove(0);
                    halo.push(dst_lnf.remove(0));
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn apply_morphism(
        &self,
        repr_tree: Arc<RwLock<ReprTree>>,
        src_type: &TypeTerm,
        dst_type: &TypeTerm
    ) {
        if let Some((m, s, σ)) = self.find_morphism_ladder( &src_type, dst_type ) {
            //eprintln!("apply morphism on subtree {:?}", s);
            let mut rt = repr_tree.descend( s ).expect("descend");
            (m.setup_projection)( &mut rt, &σ );
        } else {
            eprintln!("could not find morphism\n    {:?}\n  ====>\n    {:?}", src_type, dst_type);
        }
    }

    pub fn find_list_map_morphism<
        SrcItem: Clone + Send + Sync + 'static,
        DstItem: Clone + Send + Sync + 'static
    >(
        &self,
        mut src_item_type: TypeTerm,
        mut dst_item_type: TypeTerm
    ) -> Option<
        ( GenericReprTreeMorphism, HashMap< TypeID, TypeTerm > )
    >
    {
        if let Some((item_morphism, σ)) = self.find_morphism( &src_item_type, &dst_item_type ) {           
            (&mut src_item_type).apply_substitution( &|v| σ.get(v).clone().cloned() );
            (&mut dst_item_type).apply_substitution( &|v| σ.get(v).clone().cloned() );
       
            let src_lst_type = 
                    TypeTerm::App(vec![
                        TypeTerm::TypeID(TypeID::Fun(10 /* FIXME: remove magic */)),
                        src_item_type.clone()
                    ]);
            let dst_lst_type = 
                    TypeTerm::App(vec![
                        TypeTerm::TypeID(TypeID::Fun(10 /* FIXME: remove magic */)),
                        dst_item_type.clone()
                    ]);

            let sigmalol = σ.clone();
            let m = GenericReprTreeMorphism{
                morph_type: MorphismType {
                    src_type: src_lst_type.clone(),
                    dst_type: dst_lst_type.clone(),
                },
                setup_projection: Arc::new(move |repr_tree, subst| {
                    let src_port = repr_tree.descend( src_lst_type.clone() ).expect("descend src seq")
                        .view_list::<SrcItem>();

                    let src_item_type = src_item_type.clone();
                    let dst_item_type = dst_item_type.clone();
                    let item_morphism = item_morphism.clone();
                    let subst = subst.clone();

                    let dst_view = src_port.map(
                        move |x| {
                            let mut item_ladder = src_item_type.clone().get_lnf_vec();
                            let mut item_rt = ReprTree::from_singleton_buffer(
                                item_ladder.remove( item_ladder.len() - 1 ),
                                r3vi::buffer::singleton::SingletonBuffer::new(x.clone())
                            );

                            while item_ladder.len() > 0 {
                                let mut n = ReprTree::new_arc( item_ladder.remove( item_ladder.len() - 1) );
                                n.insert_branch( item_rt );
                                item_rt = n;
                            }

                            (item_morphism.setup_projection)( &mut item_rt, &subst );
                            item_rt.descend( dst_item_type.clone() ).expect("descend to item rt")
                                .view_singleton::< DstItem >()
                                .get_view().unwrap()
                                .get()
                        }
                    );

                    repr_tree.attach_leaf_to(
                        dst_lst_type.clone(),
                        dst_view as r3vi::view::OuterViewPort::< dyn r3vi::view::list::ListView<DstItem> >
                    );
                })
            };

            Some((m, σ))
        } else {
            eprintln!("could not find item morphism\n");
            None
        }
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>
/*
impl MorphismType {
    pub fn to_str(&self, ctx: &Context) -> String {
        format!("{:?} -> {:?}",
                if let Some(t) = self.src_type.as_ref() {
                    ctx.type_dict.read().unwrap().unparse(t)
                } else {
                    "None".into()
                },
                ctx.type_dict.read().unwrap().unparse(&self.dst_type))
    }
}
*/
