use {
    laddertypes::{TypeTerm, TypeID},
    r3vi::view::AnyOuterViewPort,
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
    ) -> Option<(&GenericReprTreeMorphism, HashMap<TypeID, TypeTerm>)> {
        for m in self.morphisms.iter() {

            let unification_problem = laddertypes::UnificationProblem::new(
                vec![
                    ( src_type.clone().normalize(), m.morph_type.src_type.clone() ),
                    ( dst_type.clone().normalize(), m.morph_type.dst_type.clone() )
                ]
            );

            let unification_result = unification_problem.solve();
            if let Ok(σ) = unification_result {
                return Some((m, σ));
            }
        }

        None
    }


    pub fn find_morphism_ladder(
        &self,
        src_type: &TypeTerm,
        dst_type: &TypeTerm,
    ) -> Option<(
        &GenericReprTreeMorphism,
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
/*
    pub fn apply_seq_map_morphism<SrcItem, DstItem>(
        &self,
        mut repr_tree: Arc<RwLock<ReprTree>>,
        src_item_type: &TypeTerm,
        dst_item_type: &TypeTerm
    ) {
        if let Some((item_morphism, σ)) = self.find_morphism( &src_item_type, dst_item_type ) {

            let src_port = repr_tree.read().unwrap().get_port::<dyn SequenceView<Item = Arc<RwLock<ReprTree>> >>().unwrap();
            src_port.map(
                m.setup_projection
            )
            
            (m.setup_projection)( &mut repr_tree, &σ );
        } else {
            eprintln!("could not find item morphism");
        }
    }
    */
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
