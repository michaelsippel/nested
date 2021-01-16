pub use {
    std::{
        sync::{Arc, RwLock},
        boxed::Box
    },
    crate::{
        core::{
            View,
            Observer,
            ObserverExt,
            ObserverBroadcast,
            ViewPort,
            InnerViewPort,
            OuterViewPort
        },
        index::{IndexView}
    }
};

impl<Key: 'static, Item: 'static> OuterViewPort<dyn IndexView<Key, Item = Item>> {
    pub fn map_item<
        DstItem: Default + 'static,
        F: Fn(&Item) -> DstItem + Send + Sync + 'static
    >(
        &self,
        f: F
    ) -> OuterViewPort<dyn IndexView<Key, Item = DstItem>> {
        let port = ViewPort::new();
        let map = MapIndexItem::new(port.inner(), f);
        self.add_observer(map.clone());
        port.into_outer()
    }
}

pub struct MapIndexItem<Key, DstItem, SrcView, F>
where SrcView: IndexView<Key> + ?Sized,
      F: Fn(&SrcView::Item) -> DstItem + Send + Sync
{
    src_view: Option<Arc<SrcView>>,
    f: F,
    cast: Arc<RwLock<ObserverBroadcast<dyn IndexView<Key, Item = DstItem>>>>
}

impl<Key, DstItem, SrcView, F> MapIndexItem<Key, DstItem, SrcView, F>
where Key: 'static,
      DstItem: Default + 'static,
      SrcView: IndexView<Key> + ?Sized + 'static,
      F: Fn(&SrcView::Item) -> DstItem + Send + Sync + 'static
{
    fn new(
        port: InnerViewPort<dyn IndexView<Key, Item = DstItem>>,
        f: F
    ) -> Arc<RwLock<Self>> {
        let map = Arc::new(RwLock::new(
            MapIndexItem {
                src_view: None,
                f,
                cast: port.get_broadcast()
            }
        ));

        port.set_view(Some(map.clone()));
        map
    }
}

impl<Key, DstItem, SrcView, F> View for MapIndexItem<Key, DstItem, SrcView, F>
where SrcView: IndexView<Key> + ?Sized,
      F: Fn(&SrcView::Item) -> DstItem + Send + Sync
{
    type Msg = Key;
}

impl<Key, DstItem, SrcView, F> IndexView<Key> for MapIndexItem<Key, DstItem, SrcView, F>
where DstItem: Default,
      SrcView: IndexView<Key> + ?Sized,
      F: Fn(&SrcView::Item) -> DstItem + Send + Sync
{
    type Item = DstItem;

    fn get(&self, key: &Key) -> Self::Item {
        if let Some(v) = self.src_view.as_ref() {
            (self.f)(&v.get(key))
        } else {
            DstItem::default()
        }
    }

    fn area(&self) -> Option<Vec<Key>> {
        self.src_view.as_ref()?.area()
    }
}

impl<Key, DstItem, SrcView, F> Observer<SrcView> for MapIndexItem<Key, DstItem, SrcView, F>
where DstItem: Default,
      SrcView: IndexView<Key> + ?Sized,
      F: Fn(&SrcView::Item) -> DstItem + Send + Sync
{
    fn reset(&mut self, view: Option<Arc<SrcView>>) {
        let old_area = self.area();
        self.src_view = view;
        let new_area = self.area();

        if let Some(area) = old_area { self.cast.notify_each(area); }
        if let Some(area) = new_area { self.cast.notify_each(area); }
    }

    fn notify(&self, msg: &Key) {
        self.cast.notify(msg);
    }
}

