

use {
    r3vi::{
        view::{
            ViewPort, OuterViewPort,
            AnyViewPort, AnyInnerViewPort, AnyOuterViewPort,
            port::UpdateTask,
            View, Observer,
            singleton::*,
            sequence::*,
            list::*
        },
        buffer::{singleton::*, vec::*}
    },
    laddertypes::{TypeTerm},
    std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        any::Any
    },
    super::{Context, ReprLeaf, ReprTreeExt}
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

#[derive(Clone)]
pub struct ReprTree {
    halo: TypeTerm,
    type_tag: TypeTerm,
    branches: HashMap<TypeTerm, Arc<RwLock<ReprTree>>>,
    leaf: Option< ReprLeaf >
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

impl std::fmt::Debug for ReprTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "| type: {:?}", self.type_tag)?;

        for (_k,x) in self.branches.iter() {
            writeln!(f, "|--> child: {:?}", x)?;
        }
        writeln!(f, "");

        Ok(())
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

impl ReprTree {
    pub fn new(type_tag: impl Into<TypeTerm>) -> Self {
        let type_tag = type_tag.into();

        assert!(type_tag.is_flat());
        
        ReprTree {
            halo: TypeTerm::unit(),
            type_tag: type_tag.clone(),
            branches: HashMap::new(),
            leaf: None
        }
    }

    pub fn new_arc(type_tag: impl Into<TypeTerm>) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self::new(type_tag)))
    }

    pub fn get_type(&self) -> &TypeTerm {
        &self.type_tag
    }

    pub fn set_halo(&mut self, halo_type: impl Into<TypeTerm>) {
        self.halo = halo_type.into();
        for (branch_type, branch) in self.branches.iter() {
            branch.write().unwrap().set_halo( TypeTerm::Ladder(vec![
                    self.halo.clone(),
                    self.type_tag.clone()
                ]).normalize()
            );
        }
    }

    pub fn get_halo_type(&self) -> &TypeTerm {
        &self.halo
    }

    pub fn get_leaf_types(&self) -> Vec< TypeTerm > {
        let mut leaf_types = Vec::new();
        if self.leaf.is_some() {
            leaf_types.push( self.get_type().clone() );
        }
        for (branch_type, branch) in self.branches.iter() {
            for t in branch.read().unwrap().get_leaf_types() {
                leaf_types.push(TypeTerm::Ladder(vec![
                    self.get_type().clone(),
                    t
                ]).normalize())
            }
        }
        leaf_types
    }

    pub fn insert_branch(&mut self, repr: Arc<RwLock<ReprTree>>) {
        let branch_type = repr.read().unwrap().get_type().clone();

        assert!(branch_type.is_flat());

        repr.write().unwrap().set_halo( TypeTerm::Ladder(vec![
            self.halo.clone(),
            self.type_tag.clone()
        ]).normalize() );

        self.branches.insert(branch_type, repr.clone());
    }

    pub fn from_char(ctx: &Arc<RwLock<Context>>, c: char ) -> Arc<RwLock<Self>> {
        ReprTree::from_singleton_buffer(
            Context::parse(ctx, "Char"),
            SingletonBuffer::new(c)
        )
    }

    pub fn from_view<V>( type_tag: impl Into<TypeTerm>, view: OuterViewPort<V> ) -> Arc<RwLock<Self>>
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        let mut rt = ReprTree::new(type_tag);
        rt.leaf = Some(ReprLeaf::from_view(view));
        Arc::new(RwLock::new(rt))
    }

    pub fn from_singleton_buffer<T>( type_tag: impl Into<TypeTerm>, buf: SingletonBuffer<T> ) -> Arc<RwLock<Self>>
    where T: Clone + Send + Sync + 'static
    {
        let mut rt = ReprTree::new(type_tag);
        rt.leaf = Some(ReprLeaf::from_singleton_buffer(buf));
        Arc::new(RwLock::new(rt))
    }

    pub fn from_vec_buffer<T>( type_tag: impl Into<TypeTerm>, buf: VecBuffer<T> ) -> Arc<RwLock<Self>>
    where T: Clone + Send + Sync + 'static
    {
        let mut rt = ReprTree::new(type_tag);
        rt.leaf = Some(ReprLeaf::from_vec_buffer(buf));
        Arc::new(RwLock::new(rt))
    }

    pub fn attach_to<V>(
        &mut self,
        src_port: OuterViewPort<V>
    )
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        if let Some(leaf) = self.leaf.as_mut() {
            leaf.attach_to(src_port);
        } else {
            eprintln!("cant attach branch without leaf");
        }
    }

    /// find, and if necessary, create corresponding path in repr-tree.
    /// Attach src_port to input of that node
    pub fn attach_leaf_to<V>(
        &mut self,
        mut type_ladder: impl Iterator<Item = TypeTerm>,
        src_port: OuterViewPort<V>
    )
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        while let Some(rung_type) = type_ladder.next() {
            if &rung_type != self.get_type() {
                if let Some(next_repr) = self.branches.get(&rung_type) {
                    next_repr.write().unwrap().attach_leaf_to(type_ladder, src_port);
                } else {
                    let mut next_repr = ReprTree::new(rung_type.clone());
                    next_repr.attach_leaf_to(type_ladder, src_port);
                    self.insert_branch(Arc::new(RwLock::new(next_repr)));
                }
                return;
            }
        }
        
        if let Some(leaf) = self.leaf.as_mut() {
            leaf.attach_to(src_port);
        } else {
            if self.type_tag == TypeTerm::App(vec![
                TypeTerm::TypeID(TypeID::Fun(11)),
                TypeTerm::TypeID(TypeID::Fun(2))
            ]) {
                let mut leaf = ReprLeaf::from_vec_buffer(
                    VecBuffer::<
                        Arc<RwLock<crate::edit_tree::EditTree>>
                    >::new()
                );

                leaf.attach_to(src_port);
                self.leaf = Some(leaf);
            } else {
                self.leaf = Some(ReprLeaf::from_view(src_port));
            }
        }
    }

    pub fn detach(&mut self, ctx: &Arc<RwLock<Context>>) {
        if let Some(leaf) = self.leaf.as_mut() {
            if self.type_tag == Context::parse(&ctx, "Char") {
                leaf.detach::<dyn SingletonView<Item = char>>();
            }
            if self.type_tag == Context::parse(&ctx, "<Vec Char>") {
                leaf.detach_vec::<char>();
            }
            if self.type_tag == Context::parse(&ctx, "<List Char>") {
                leaf.detach::<dyn ListView<char>>();
            }
        }

        for (t,b) in self.branches.iter_mut() {
            b.write().unwrap().detach(&ctx);
        }
    }

    pub fn insert_leaf(
        &mut self,
        mut type_ladder: impl Iterator<Item = TypeTerm>,
        leaf: ReprLeaf
    ) {
        while let Some(type_term) = type_ladder.next() {
            if &type_term != self.get_type() {
                if let Some(next_repr) = self.branches.get(&type_term) {
                    next_repr.write().unwrap().insert_leaf(type_ladder, leaf.clone());
                } else {
                    let mut next_repr = ReprTree::new(type_term.clone());
                    next_repr.insert_leaf(type_ladder, leaf.clone());
                    self.insert_branch(Arc::new(RwLock::new(next_repr)));
                }
                return;
            }
        }

        self.leaf = Some(leaf);
    }

    //<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

    pub fn descend_one(&self, dst_type: impl Into<TypeTerm>) -> Option<Arc<RwLock<ReprTree>>> {
        let dst_type = dst_type.into();
        assert!( dst_type.is_flat() );
        self.branches.get(&dst_type).cloned()
    }

    pub fn descend_ladder(rt: &Arc<RwLock<Self>>, mut repr_ladder: impl Iterator<Item = TypeTerm>) -> Option<Arc<RwLock<ReprTree>>> {
        if let Some(first) = repr_ladder.next() {
            let rt = rt.read().unwrap();
            repr_ladder.fold(
                rt.descend_one(first),
                |s, t| s?.descend(t))
        } else {
            Some(rt.clone())
        }
    }

    pub fn descend(rt: &Arc<RwLock<Self>>, dst_type: impl Into<TypeTerm>) -> Option<Arc<RwLock<ReprTree>>> {
        let mut lnf = dst_type.into().get_lnf_vec();
        if lnf.len() > 0 {
            if lnf[0] == rt.get_type() {
                lnf.remove(0);
            }
            ReprTree::descend_ladder(rt, lnf.into_iter())
        } else {
            Some(rt.clone())
        }
    }

    pub fn ascend(rt: &Arc<RwLock<Self>>, type_term: impl Into<TypeTerm>) -> Arc<RwLock<ReprTree>> {
        let mut n = Self::new(type_term);
        n.insert_branch(rt.clone());
        Arc::new(RwLock::new(n))
    }

    //<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

    pub fn singleton_buffer<T: Clone + Send + Sync + 'static>(&mut self) -> Option<SingletonBuffer<T>> {
        if let Some(leaf) = self.leaf.as_mut() {
            leaf.as_singleton_buffer::<T>()
        } else {
            // create new singleton buffer
            /*
            // default value??
            let buf = SingletonBuffer::<T>::default();
            self.leaf = Some(ReprLeaf::from_singleton_buffer(buf.clone()));
            Some(buf)
            */
            None
        }
    }

    pub fn vec_buffer<T: Clone + Send + Sync + 'static>(&mut self) -> Option<VecBuffer<T>> {
        if let Some(leaf) = self.leaf.as_mut() {
            leaf.as_vec_buffer::<T>()
        } else {
            None
        }
    }

    //<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

    pub fn get_port<V: View + ?Sized + 'static>(&self) -> Option<OuterViewPort<V>>
    where
        V::Msg: Clone,
    {
        if let Some(leaf) = self.leaf.as_ref() {
            leaf.get_port::<V>()
        } else {
            None
        }
    }

    pub fn get_view<V: View + ?Sized + 'static>(&self) -> Option<Arc<V>>
    where
        V::Msg: Clone,
    {
        self.get_port::<V>()?
            .get_view()
    }

    //<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

    pub fn view_singleton<T: 'static>(&self) -> OuterViewPort<dyn SingletonView<Item = T>> {
        self.get_port::<dyn SingletonView<Item = T>>().expect("no singleton-view available")
    }

    pub fn view_seq<T: 'static>(&self) -> OuterViewPort<dyn SequenceView<Item = T>> {
        self.get_port::<dyn SequenceView<Item = T>>().expect("no sequence-view available")
    }

    pub fn view_list<T: Clone + Send + Sync + 'static>(&self) -> OuterViewPort<dyn ListView<T>> {
        self.get_port::<dyn ListView<T>>().expect("no list-view available")
    }

    pub fn view_char(&self) -> OuterViewPort<dyn SingletonView<Item = char>> {
        self.get_port::<dyn SingletonView<Item = char>>().expect("no char-view available")
    }

    pub fn view_u8(&self) -> OuterViewPort<dyn SingletonView<Item = u8>> {
        self.get_port::<dyn SingletonView<Item = u8>>().expect("no u8-view available")
    }

    pub fn view_u64(&self) -> OuterViewPort<dyn SingletonView<Item = u64>> {
        self.get_port::<dyn SingletonView<Item = u64>>().expect("no u64-view available")
    }

    pub fn view_usize(&self) -> OuterViewPort<dyn SingletonView<Item = usize>> {
        self.get_port::<dyn SingletonView<Item = usize>>().expect("no usize-view available")
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

