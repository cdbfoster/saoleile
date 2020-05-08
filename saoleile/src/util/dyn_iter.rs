pub trait DynIter<'a> {
    type Item;

    fn dyn_iter(&'a self) -> Box<dyn Iterator<Item = &'a Self::Item> + 'a>;
}