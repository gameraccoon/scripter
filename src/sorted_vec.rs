use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct SortedVec<T> {
    data: Vec<T>,
}

impl<T> Deref for SortedVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> SortedVec<T>
where
    T: Ord,
{
    pub fn from_one_value(value: T) -> Self {
        Self { data: vec![value] }
    }

    pub fn remove_sorted(&mut self, value: &T) {
        if let Ok(idx_to_remove) = self.data.binary_search(value) {
            self.data.remove(idx_to_remove);
        }
    }

    pub fn insert_unique_sorted(&mut self, value: T) {
        if let Err(insertion_idx) = self.data.binary_search(&value) {
            self.data.insert(insertion_idx, value);
        }
    }

    pub fn unsafe_modify(&mut self, modify: impl Fn(&mut Vec<T>)) {
        modify(&mut self.data);

        if !self.data.is_sorted() {
            panic!("The vec should be sorted after performing unsafe operations");
        }
    }
}
