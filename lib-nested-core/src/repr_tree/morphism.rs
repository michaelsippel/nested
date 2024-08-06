use {
    laddertypes::{TypeTerm, TypeID, morphism::Morphism},
    r3vi::view::{AnyOuterViewPort, port::UpdateTask},
    crate::{
        repr_tree::{ReprTree, ReprTreeExt, ReprLeaf},
    },
    std::{
        sync::{Arc, RwLock},
        collections::HashMap
    }
};

pub use laddertypes::morphism::MorphismType;

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

#[derive(Clone)]
pub struct GenericReprTreeMorphism {
    pub(super) morph_type: MorphismType,
    pub(super) setup_projection: Arc<
        dyn Fn( &mut Arc<RwLock<ReprTree>>, &HashMap<TypeID, TypeTerm> )
//            -> Result< ReprLeaf, () >
        + Send + Sync
    >
}

impl Morphism for GenericReprTreeMorphism {
    fn get_type(&self) -> MorphismType {
        self.morph_type.clone()
    }

    fn list_map_morphism(&self, list_typeid: TypeID) -> Option< GenericReprTreeMorphism > {
        self.into_list_map_dyn(list_typeid)
    }
}

impl GenericReprTreeMorphism {
    pub fn new(
        src_type: TypeTerm,
        dst_type: TypeTerm,

        setup: impl Fn( &mut Arc<RwLock<ReprTree>>, &HashMap<TypeID, TypeTerm> )
                + Send + Sync + 'static
    ) -> Self {
        GenericReprTreeMorphism {
            morph_type: MorphismType {
                src_type, dst_type
            }.normalize(),

            setup_projection: Arc::new(setup)
        }
    }

    pub fn into_list_map< SrcItem, DstItem >(&self, list_typeid: TypeID)
    -> GenericReprTreeMorphism
    where
        SrcItem: Clone + Send + Sync + 'static,
        DstItem: Clone + Send + Sync + 'static
    {
        let mut lst_map_type = MorphismType {
            src_type: TypeTerm::App(vec![
                TypeTerm::TypeID(list_typeid),
                self.morph_type.src_type.clone()
            ]),
            dst_type: TypeTerm::App(vec![
                TypeTerm::TypeID(list_typeid),
                self.morph_type.dst_type.clone()
            ])
        }.normalize();

        let item_morph = self.clone();

        GenericReprTreeMorphism{
            morph_type: lst_map_type.clone(),
            setup_projection: Arc::new(move |repr_tree, subst| {
                let mut lst_map_type = lst_map_type.clone();
                lst_map_type.src_type.apply_substitution( &|x| subst.get(x).cloned() );
                lst_map_type.dst_type.apply_substitution( &|x| subst.get(x).cloned() );

                eprintln!(
                    "lst map type :  {:?}", lst_map_type
                );

                let src_port = repr_tree
                    .descend( lst_map_type.src_type.clone() )
                    .expect("descend src seq")
                    .view_list::<SrcItem>();

                let subst = subst.clone();
                let item_morph = item_morph.clone();

                let dst_view = src_port.map(
                        move |x| {
                            let mut item_ladder = item_morph.morph_type.src_type.clone().get_lnf_vec();
                            let mut item_rt = ReprTree::from_singleton_buffer(
                                item_ladder.remove( item_ladder.len() - 1 ),
                                r3vi::buffer::singleton::SingletonBuffer::new(x.clone())
                            );

                            // TODO: required?
                            while item_ladder.len() > 0 {
                                let mut n = ReprTree::new_arc( item_ladder.remove( item_ladder.len() - 1) );
                                n.insert_branch( item_rt );
                                item_rt = n;
                            }

                            (item_morph.setup_projection)( &mut item_rt, &subst );
                            item_rt.descend( item_morph.morph_type.dst_type.clone() ).expect("descend to item rt")
                                .view_singleton::< DstItem >()
                                .get_view().unwrap()
                                .get()
                        }
                );

                repr_tree.attach_leaf_to(
                    lst_map_type.dst_type.clone(),
                    dst_view as r3vi::view::OuterViewPort::< dyn r3vi::view::list::ListView<DstItem> >
                );
            })
        }
    }

    pub fn into_list_map_dyn(&self, typeid_list: TypeID)
    -> Option< GenericReprTreeMorphism >
    { 
        let typeid_char = TypeID::Fun(1);
        let typeid_u64 = TypeID::Fun(5);
        let typeid_edittree = TypeID::Fun(2);

        let src_item_type_lnf = self.morph_type.src_type.clone().get_lnf_vec();
        let dst_item_type_lnf = self.morph_type.dst_type.clone().get_lnf_vec();

        if src_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_char)) &&
           dst_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_u64))
        {
            Some( self.into_list_map::< char, u64 >(typeid_list) )
        }
        else if src_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_u64)) &&
                dst_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_char))
        {
            Some( self.into_list_map::< u64, char >(typeid_list) )
        }
        else if src_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_char)) &&
                dst_item_type_lnf.last() == Some(&TypeTerm::TypeID(typeid_edittree))
        {
            Some( self.into_list_map::< char, Arc<RwLock<crate::edit_tree::EditTree>> >(typeid_list) )
        }
        else
        {
            eprintln!("no list map type for {:?}", dst_item_type_lnf.last());
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
