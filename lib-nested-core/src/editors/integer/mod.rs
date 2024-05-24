pub mod add;
pub mod editor;
pub mod radix;
pub mod ctx;

pub use {
    add::Add,
    editor::PosIntEditor,
    radix::RadixProjection,
    ctx::init_ctx
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

use {
    r3vi::{
        view::{
            View, ViewPort, OuterViewPort,
            Observer,
            ObserverBroadcast,
            sequence::*
        }
    },
    crate::{
        editors::integer::radix::{
            PosIntProjections
        }
    },
    std::sync::{Arc, RwLock}
};

pub trait PositionalUInt : SequenceView<Item = usize> {
    fn get_radix(&self) -> usize;
    fn get_value(&self) -> usize {
        let mut val = 0;
        let mut r = 1;
        for i in 0..self.len().unwrap_or(0) {
            val += r * self.get(&i).unwrap();
            r *= self.get_radix();
        }

        val  
    }
}

impl<V: PositionalUInt> PositionalUInt for RwLock<V> {
    fn get_radix(&self) -> usize {
        self.read().unwrap().get_radix()
    }
}

struct PosUIntFromDigits {
    radix: usize,
    src_digits: Option<Arc<dyn SequenceView<Item = usize>>>,
    cast: Arc<RwLock<ObserverBroadcast<dyn PositionalUInt>>>
}

impl View for PosUIntFromDigits {
    type Msg = usize;
}

impl SequenceView for PosUIntFromDigits {
    type Item = usize;

    fn get(&self, idx: &usize) -> Option<usize> {
        self.src_digits.get(idx)
    }

    fn len(&self) -> Option<usize> {
        self.src_digits.len()
    }
}

impl PositionalUInt for PosUIntFromDigits {
    fn get_radix(&self) -> usize {
        self.radix
    }
}

impl Observer<dyn SequenceView<Item = usize>> for PosUIntFromDigits {
    fn reset(&mut self, new_src: Option<Arc<dyn SequenceView<Item = usize>>>) {
        self.src_digits = new_src;
//        self.cast.write().unwrap().notify(0);
    }

    fn notify(&mut self, idx: &usize) {
        self.cast.write().unwrap().notify(idx);
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub trait DigitSeqProjection {
    fn to_positional_uint(&self, radix: usize) -> OuterViewPort<dyn PositionalUInt>;
}

impl DigitSeqProjection for OuterViewPort<dyn SequenceView<Item = usize>> {
    fn to_positional_uint(&self, radix: usize) -> OuterViewPort<dyn PositionalUInt> {
        let port = ViewPort::new();
        port.add_update_hook(Arc::new(self.0.clone()));

        let proj = Arc::new(RwLock::new(PosUIntFromDigits {
            radix,
            src_digits: None,
            cast: port.inner().get_broadcast()
        }));

        self.add_observer(proj.clone());
        port.set_view(Some(proj));
        port.into_outer()
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

struct PosUIntToDigits {
    src: Option<Arc<dyn PositionalUInt>>,
    cast: Arc<RwLock<ObserverBroadcast<dyn SequenceView<Item = usize>>>>
}

impl View for PosUIntToDigits {
    type Msg = usize;
}

impl SequenceView for PosUIntToDigits {
    type Item = usize;

    fn get(&self, idx: &usize) -> Option<usize> {
        self.src.get(idx)
    }

    fn len(&self) -> Option<usize> {
        self.src.len()
    }
}

impl Observer<dyn PositionalUInt> for PosUIntToDigits {
    fn reset(&mut self, view: Option<Arc<dyn PositionalUInt>>) {
        self.src = view;
//        self.cast.notify_all();
    }

    fn notify(&mut self, idx: &usize) {
        self.cast.notify(idx);
    }
}


