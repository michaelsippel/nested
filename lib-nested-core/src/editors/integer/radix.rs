use {
    r3vi::{
        view::{
            View, ViewPort,
            InnerViewPort, Observer, OuterViewPort,
            ObserverBroadcast,
            sequence::*,
            list::*
        },
        buffer::{vec::*}
    },
    crate::{
        editors::integer::{
            PositionalUInt
        },
        repr_tree::{ReprTree, ReprLeaf}
    },
    std::sync::{Arc, RwLock},
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub trait PosIntProjections {
    fn transform_radix(&self, dst_radix: u64) -> OuterViewPort<dyn SequenceView<Item = u64>>;
//    fn to_digits(&self) -> OuterViewPort<dyn SequenceView<Item = usize>>;
}

impl PosIntProjections for OuterViewPort<dyn PositionalUInt> {
    fn transform_radix(&self, dst_radix: u64) -> OuterViewPort<dyn SequenceView<Item = u64>> {
        let port = ViewPort::<dyn SequenceView<Item = u64>>::new();
        port.add_update_hook(Arc::new(self.0.clone()));

//        let mut vec_port = ViewPort::new();
        let proj = Arc::new(RwLock::new(RadixProjection {
            src: None,
            dst_radix,
            dst_digits: VecBuffer::new(),
            cast: port.inner().get_broadcast()
        }));

        self.add_observer(proj.clone());
        port.set_view(Some(proj as Arc<dyn SequenceView<Item = u64>>));
        port.into_outer()
    }
/*
    fn to_digits(&self) -> OuterViewPort<dyn SequenceView<Item = usize>> {
        let port = ViewPort::new();
        port.add_update_hook(Arc::new(self.0.clone()));
        let proj = Arc::new(RwLock::new(PosUIntToDigits {
            src: None,
            cast: port.inner().get_broadcast()
        }));
        self.add_observer(proj.clone());
        port.inner().set_view(Some(proj));
        port.into_outer()
    }
    */
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

pub struct RadixProjection {
    src: Option<Arc<dyn PositionalUInt>>,
    dst_radix: u64,
    dst_digits: VecBuffer<u64>,
    cast: Arc<RwLock<ObserverBroadcast<dyn SequenceView<Item = u64>>>>
}

impl View for RadixProjection {
    type Msg = usize;
}

impl SequenceView for RadixProjection {
    type Item = u64;

    fn get(&self, idx: &usize) -> Option<u64> {
        if *idx < self.dst_digits.len() {
            Some(self.dst_digits.get(*idx))
        } else {
            None
        }
    }

    fn len(&self) -> Option<usize> {
        Some(self.dst_digits.len())
    }
}

impl PositionalUInt for RadixProjection {
    fn get_radix(&self) -> u64 {
        self.dst_radix
    }
}

impl Observer< dyn PositionalUInt > for RadixProjection {
    fn reset(&mut self, view: Option<Arc<dyn PositionalUInt>>) {
        self.src = view;
        self.update();
    }

    fn notify(&mut self, idx: &usize) {
        self.update();
        // self.update_digit(idx)
    }
}

impl RadixProjection {
    /// recalculate everything
    fn update(&mut self) {
//       let mut dst = self.dst_digits;
        let old_len = self.dst_digits.len();
        self.dst_digits.clear();

        if let Some(src) = self.src.as_ref() {
            let mut val = src.get_value();
            while val > 0 {
                self.dst_digits.push(val % self.dst_radix);
                val /= self.dst_radix;
            }
        }

        let new_len = self.dst_digits.len();
        for i in 0 .. usize::max(old_len, new_len) {
             self.cast.write().unwrap().notify(&i);
        }
    }

    fn _update_dst_digit(&mut self, _idx: usize) {
        /*
                let v = 0; // calculate new digit value

                // which src-digits are responsible?

                if idx < self.dst_digits.len() {
                    self.dst_digits.get_mut(idx) = v;
                } else if idx == self.dst_digits.len() {
                    self.dst_digits.push(v);
                } else {
                    // error
                }
        */
    }
}

