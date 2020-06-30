use std::collections::BTreeMap;

struct IndexSet<T>(Vec<T>);

impl<T> Default for IndexSet<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> IndexSet<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get<U>(&mut self, value: &U) -> usize
    where
        for<'a> &'a T: PartialEq<&'a U>,
        T: std::borrow::Borrow<U>,
        U: std::borrow::ToOwned<Owned = T>,
    {
        for (index, item) in self.0.iter().enumerate() {
            if item == value {
                return index;
            }
        }
        let index = self.0.len();
        self.0.push(value.to_owned());
        index
    }
}

struct RenderGraph {
    names: IndexSet<String>,
    buffers: BTreeMap<usize, usize>,
    textures: BTreeMap<usize, usize>,
    passes: RenderGraphPass,
}

struct RenderGraphPass {
    inputs: Vec<usize>,
    outputs: Vec<usize>,
}
