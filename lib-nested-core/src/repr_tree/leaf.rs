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

#[derive(Clone)]
pub struct ReprLeaf {
    out_port: AnyViewPort,
    in_port: AnyInnerViewPort,
    data: Option< Arc<dyn Any + Send + Sync> >,

    /// keepalive for the observer that updates the buffer from in_port
    keepalive: Option<Arc<dyn Any + Send + Sync>>,
    in_keepalive: Option<Arc<dyn Any + Send + Sync>>,
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

impl ReprLeaf {
    pub fn from_view<V>( src_port: OuterViewPort<V> ) -> Self
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        let mut in_port = ViewPort::<V>::new();
        let in_keepalive = in_port.attach_to(src_port);

        let mut out_port = ViewPort::<V>::new();
        let out_keepalive = out_port.attach_to(in_port.outer());

        ReprLeaf {
            keepalive: Some(out_keepalive),
            in_keepalive: Some(in_keepalive),
            in_port: in_port.inner().into(),
            out_port: out_port.into(),
            data: None, 
        }
    }

    pub fn detach<V>(&mut self)
    where V: View + ?Sized + 'static,
         V::Msg: Clone
    {
        self.keepalive = None;
        self.in_keepalive = None;

        let ip = self.in_port.clone()
            .downcast::<V>().ok()
            .unwrap();
        ip.0.detach();

        if self.data.is_none() {
            let mut op = self.out_port.clone()
                .downcast::<V>().ok()
                .unwrap();

            op.detach();
            self.keepalive = Some(op.attach_to(ip.0.outer()));
        }
    }

    pub fn detach_vec<Item>(&mut self)
    where Item: Clone + Send + Sync + 'static
    {
        self.keepalive = None;
        self.in_keepalive = None;

        let ip = self.in_port.clone()
            .downcast::<dyn ListView<Item>>().ok()
            .unwrap();

        ip.0.detach();

        if let Some(data) = self.data.as_mut() {
            let mut op = self.out_port.clone()
                .downcast::<RwLock<Vec<Item>>>().ok()
                .unwrap();
            op.detach();

            let data = data.clone().downcast::< RwLock<Vec<Item>> >().ok().unwrap();
            let buffer = VecBuffer::with_data_arc_port(data, op.inner());
            self.keepalive = Some(buffer.attach_to(ip.0.outer()))
        }
    }

    pub fn attach_to<V>(&mut self, src_port: OuterViewPort<V>)
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        self.in_keepalive = Some(self.in_port.clone()
            .downcast::<V>().ok().unwrap()
            .0.attach_to( src_port ));
    }

    pub fn from_singleton_buffer<T>( buffer: SingletonBuffer<T> ) -> Self
    where T: Clone + Send + Sync + 'static
    {
        let in_port = ViewPort::<dyn SingletonView<Item = T>>::new();
        ReprLeaf {
            in_keepalive: None,
            keepalive: Some(buffer.attach_to(in_port.outer())),
            in_port: in_port.inner().into(),
            out_port: buffer.get_port().0.into(),
            data: Some(buffer.into_inner())
        }
    }

    pub fn from_vec_buffer<T>( buffer: VecBuffer<T> ) -> Self
    where T: Clone + Send + Sync + 'static
    {
        let in_port = ViewPort::< dyn ListView<T> >::new();
        ReprLeaf {
            in_keepalive: None,
            keepalive: Some(buffer.attach_to(in_port.outer())),
            in_port: in_port.inner().into(),
            out_port: buffer.get_port().0.into(),
            data: Some(buffer.into_inner())
        }
    }

    pub fn as_singleton_buffer<T>(&mut self) -> Option<SingletonBuffer<T>>
    where T: Clone + Send + Sync + 'static
    {
        let sgl_port = self.get_port::< dyn SingletonView<Item = T> >().unwrap().0;

        let data_arc =
            if let Some(data) = self.data.as_ref() {
                data.clone().downcast::<RwLock<T>>().ok()
            } else {
                sgl_port.update();
                let value = sgl_port.outer().get_view().unwrap().get();
                eprintln!("make new data ARC from old value");
                Some(Arc::new(RwLock::new( value )))
            };

        if let Some(data_arc) = data_arc {
            self.data = Some(data_arc.clone() as Arc<dyn Any + Send + Sync>);
            let buf = SingletonBuffer {
                value: data_arc,
                port: sgl_port.inner()
            };
            self.keepalive = Some(buf.attach_to(
                self.in_port.0.clone()
                    .downcast::<dyn SingletonView<Item = T>>()
                    .ok().unwrap()
                    .outer()
            ));
            Some(buf)
        } else {
            None
        }
    }

    pub fn as_vec_buffer<T>(&mut self) -> Option<VecBuffer<T>>
    where T: Clone + Send + Sync + 'static
    {
        let vec_port = self.get_port::< RwLock<Vec<T>> >().unwrap().0;

        let data_arc =
            if let Some(data) = self.data.as_ref() {
                data.clone().downcast::<RwLock<Vec<T>>>().ok()
            } else {
                vec_port.update();
                if let Some(value) = vec_port.outer().get_view() {
                    let value = value.read().unwrap().clone();
                    eprintln!("make new data ARC from old VECTOR-value");
                    Some(Arc::new(RwLock::new( value )))
                } else {
                    eprintln!("no data vec");
                    Some(Arc::new(RwLock::new( Vec::new() )))
//                    None
                }
            };

        if let Some(data_arc) = data_arc {
            self.data = Some(data_arc.clone() as Arc<dyn Any + Send + Sync>);
            let buf = VecBuffer::with_data_arc_port(data_arc, vec_port.inner());
            self.keepalive = Some(buf.attach_to(
                self.in_port.0.clone()
                    .downcast::< dyn ListView<T> >()
                    .ok().unwrap()
                    .outer()
            ));
            Some(buf)
        } else {
            None
        }
    }

    pub fn get_port<V>(&self) -> Option<OuterViewPort<V>>
    where V: View + ?Sized + 'static,
        V::Msg: Clone
    {
        self.out_port.clone().downcast::<V>().ok().map(|p| p.outer())
    }
}

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

