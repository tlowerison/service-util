pub trait Id {
    type Id: Clone;
    fn id(&self) -> Self::Id;
}

pub trait WithVariantUpdate: Id + Sized
where
    Self::Id: std::hash::Hash,
{
    type Variant: Split;
    type Split;

    fn split(self, existing: &std::collections::HashMap<Self::Id, Self>) -> Self::Split;
}

pub trait Split: Sized {
    type Components: From<Self>;
    fn split(self) -> Self::Components;
}
