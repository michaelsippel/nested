pub mod node;
pub mod leaf;
pub mod context;
pub mod morphism;

#[cfg(test)]
mod tests;

pub use {
    context::{Context},
    leaf::ReprLeaf,
    node::ReprTree,
    morphism::{GenericReprTreeMorphism}
};

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
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub trait ReprTreeExt {
    fn get_type(&self) -> TypeTerm;

    fn insert_leaf(&mut self, type_ladder: impl Into<TypeTerm>, leaf: ReprLeaf);
    fn insert_branch(&mut self, repr: Arc<RwLock<ReprTree>>);
    fn create_branch(&mut self, rung: impl Into<TypeTerm>);
    fn descend(&self, target_type: impl Into<TypeTerm>) -> Option<Arc<RwLock<ReprTree>>>;

    fn attach_leaf_to<V: View + ?Sized + 'static>(&self, t: impl Into<TypeTerm>, v: OuterViewPort<V>) where V::Msg: Clone;
    fn get_port<V: View + ?Sized + 'static>(&self) -> Option<OuterViewPort<V>> where V::Msg: Clone;

    fn view_char(&self) -> OuterViewPort<dyn SingletonView<Item = char>>;
    fn view_u8(&self) -> OuterViewPort<dyn SingletonView<Item = u8>>;
    fn view_u64(&self) -> OuterViewPort<dyn SingletonView<Item = u64>>;
    fn view_usize(&self) -> OuterViewPort<dyn SingletonView<Item = usize>>;

    fn view_singleton<T: Send + Sync + 'static>(&self) -> OuterViewPort<dyn SingletonView<Item = T>>;
    fn view_seq<T: Send + Sync + 'static>(&self) -> OuterViewPort<dyn SequenceView<Item = T>>;
    fn view_list<T: Clone + Send + Sync + 'static>(&self) -> OuterViewPort<dyn ListView<T>>;

    fn singleton_buffer<T: Clone + Send + Sync + 'static>(&self) -> SingletonBuffer<T>;
    fn vec_buffer<T: Clone + Send + Sync + 'static>(&self) -> VecBuffer<T>;
}

impl ReprTreeExt for Arc<RwLock<ReprTree>> {
    fn get_type(&self) -> TypeTerm {
        self.read().unwrap().get_type().clone()
    }

    fn insert_leaf(&mut self, type_ladder: impl Into<TypeTerm>, leaf: ReprLeaf) {
        self.write().unwrap().insert_leaf(type_ladder.into().get_lnf_vec().into_iter(), leaf)
    }

    fn insert_branch(&mut self, repr: Arc<RwLock<ReprTree>>) {
        self.write().unwrap().insert_branch(repr)
    }

    fn create_branch(&mut self, rung: impl Into<TypeTerm>) {
        let mut lnf = rung.into().get_lnf_vec().into_iter();
        if let Some(rung) = lnf.next() {
            let mut parent = ReprTree::new_arc( rung );
            self.insert_branch( parent.clone() );

            for rung in lnf {
                let r = ReprTree::new_arc( rung );
                parent.insert_branch(r.clone());
                parent = r;
            }
        }
    }

    fn get_port<V: View + ?Sized + 'static>(&self) -> Option<OuterViewPort<V>> where V::Msg: Clone {
        self.read().unwrap().get_port::<V>()
    }

    fn attach_leaf_to<V: View + ?Sized + 'static>(&self, type_ladder: impl Into<TypeTerm>, v: OuterViewPort<V>) where V::Msg: Clone {
        self.write().unwrap().attach_leaf_to::<V>(type_ladder.into().get_lnf_vec().into_iter(), v)
    }

    fn descend(&self, target_type: impl Into<TypeTerm>) -> Option<Arc<RwLock<ReprTree>>> {
        ReprTree::descend( self, target_type )
    }

    fn view_char(&self) -> OuterViewPort<dyn SingletonView<Item = char>> {
        self.read().unwrap().view_char()
    }

    fn view_u8(&self) -> OuterViewPort<dyn SingletonView<Item = u8>> {
        self.read().unwrap().view_u8()
    }

    fn view_u64(&self) -> OuterViewPort<dyn SingletonView<Item = u64>> {
        self.read().unwrap().view_u64()
    }

    fn view_usize(&self) -> OuterViewPort<dyn SingletonView<Item = usize>> {
        self.read().unwrap().view_usize()
    }

    fn view_singleton<T: Send + Sync + 'static>(&self) -> OuterViewPort<dyn SingletonView<Item = T>> {
        self.read().unwrap().view_singleton::<T>()
    }

    fn view_seq<T: Send + Sync + 'static>(&self) -> OuterViewPort<dyn SequenceView<Item = T>> {
        self.read().unwrap().view_seq::<T>()
    }

    fn view_list<T: Clone + Send + Sync + 'static>(&self) -> OuterViewPort<dyn ListView<T>> {
        self.read().unwrap().view_list::<T>()
    }

    fn singleton_buffer<T: Clone + Send + Sync + 'static>(&self) -> SingletonBuffer<T> {
        self.write().unwrap().singleton_buffer::<T>().expect("")
    }

    fn vec_buffer<T: Clone + Send + Sync + 'static>(&self) -> VecBuffer<T> {
        self.write().unwrap().vec_buffer::<T>().expect("")
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

