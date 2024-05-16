// Copyright (C) Pavel Grebnev 2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

/// An implementation of a sparse set, a data structure that stores a set of items and
/// provides a way to efficiently access them by index.
/// It allows for constant time insertions and lookups
/// Good for cache efficiency, doesn't require any hashing,
/// But it is NOT scalable for large data sets
/// The used memory only grows, can free it only by destroying the whole set
#[derive(Clone)]
pub struct SparseSet<T> {
    // has as many values as elements stored in the set
    dense: Vec<T>,
    // same size as the dense array, stores back references to the sparse array
    dense_index_to_sparse_index: Vec<usize>,
    // stores either index to the value in the dense array or index to the next free sparse slot
    sparse: Vec<SparseEntry>,
    // a "free list" of free entries in the sparse array
    next_free_sparse_entry: usize,
}

/// The index should be stored runtime, it doesn't make sense to serialize it since
/// the SparseSet data is not persistent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
    sparse_index: usize,
    epoch: usize,
}

#[derive(Clone)]
enum SparseEntry {
    AliveEntry(AliveSparseEntry),
    FreeEntry(FreeSparseEntry),
}

#[derive(Clone)]
struct AliveSparseEntry {
    dense_index: usize,
    epoch: usize,
}

#[derive(Clone)]
struct FreeSparseEntry {
    next_free: usize,
    next_epoch: usize,
}

#[allow(dead_code)]
impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            dense_index_to_sparse_index: Vec::new(),
            sparse: Vec::new(),
            next_free_sparse_entry: usize::MAX,
        }
    }

    pub fn push(&mut self, value: T) -> Index {
        let dense_index = self.dense.len();

        // if there are free entry in the sparse array, use one of them
        let index = if self.next_free_sparse_entry != usize::MAX {
            let new_sparse_index = self.next_free_sparse_entry;
            let free_sparse_entry = match &self.sparse[new_sparse_index] {
                SparseEntry::FreeEntry(free_sparse_entry) => free_sparse_entry.clone(),
                _ => unreachable!(),
            };
            self.next_free_sparse_entry = free_sparse_entry.next_free;

            self.sparse[new_sparse_index] = SparseEntry::AliveEntry(AliveSparseEntry {
                dense_index,
                epoch: free_sparse_entry.next_epoch,
            });

            Index {
                sparse_index: new_sparse_index,
                epoch: free_sparse_entry.next_epoch,
            }
        } else {
            // extend the sparse array
            self.sparse.push(SparseEntry::AliveEntry(AliveSparseEntry {
                dense_index,
                epoch: 0,
            }));

            Index {
                sparse_index: self.sparse.len() - 1,
                epoch: 0,
            }
        };

        self.dense.push(value);
        self.dense_index_to_sparse_index.push(index.sparse_index);

        index
    }

    pub fn remove_swap(&mut self, index: &Index) -> Option<T> {
        // this can happen only if the index is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(index.sparse_index < self.sparse.len());

        return match self.sparse[index.sparse_index].clone() {
            SparseEntry::AliveEntry(entry) if entry.epoch == index.epoch => {
                let removed_value = self.dense.swap_remove(entry.dense_index);
                let swapped_sparse_index = self.dense_index_to_sparse_index[self.dense.len()];
                self.dense_index_to_sparse_index
                    .swap_remove(entry.dense_index);

                if let SparseEntry::AliveEntry(swapped_entry) =
                    &mut self.sparse[swapped_sparse_index]
                {
                    swapped_entry.dense_index = entry.dense_index;
                } else {
                    unreachable!();
                }

                self.sparse[index.sparse_index] = SparseEntry::FreeEntry(FreeSparseEntry {
                    next_free: self.next_free_sparse_entry,
                    next_epoch: usize::wrapping_add(entry.epoch, 1),
                });

                // as long as we have available epochs, we can reuse the sparse index
                if index.epoch < usize::MAX {
                    self.next_free_sparse_entry = index.sparse_index;
                }

                Some(removed_value)
            }
            // the element was already removed (either there's nothing, or a newer element)
            _ => None,
        };
    }

    pub fn remove_stable(&mut self, index: &Index) -> Option<T> {
        // this can happen only if the index is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(index.sparse_index < self.sparse.len());

        return match self.sparse[index.sparse_index].clone() {
            SparseEntry::AliveEntry(entry) if entry.epoch == index.epoch => {
                for i in entry.dense_index + 1..self.dense.len() {
                    let sparse_index = self.dense_index_to_sparse_index[i];
                    if let SparseEntry::AliveEntry(entry) = &mut self.sparse[sparse_index] {
                        entry.dense_index -= 1;
                    } else {
                        unreachable!();
                    }
                }
                let removed_value = self.dense.remove(entry.dense_index);
                self.dense_index_to_sparse_index.remove(entry.dense_index);

                self.sparse[index.sparse_index] = SparseEntry::FreeEntry(FreeSparseEntry {
                    next_free: self.next_free_sparse_entry,
                    next_epoch: usize::wrapping_add(entry.epoch, 1),
                });

                // as long as we have available epochs, we can reuse the sparse index
                if index.epoch < usize::MAX {
                    self.next_free_sparse_entry = index.sparse_index;
                }
                Some(removed_value)
            }
            // the element was already removed (either there's nothing, or a newer element)
            _ => None,
        };
    }

    pub fn swap(&mut self, index1: &Index, index2: &Index) {
        // this can happen only if the index is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(index1.sparse_index < self.sparse.len());
        assert!(index2.sparse_index < self.sparse.len());

        match (
            self.sparse[index1.sparse_index].clone(),
            self.sparse[index2.sparse_index].clone(),
        ) {
            (SparseEntry::AliveEntry(entry1), SparseEntry::AliveEntry(entry2))
                if entry1.epoch == index1.epoch && entry2.epoch == index2.epoch =>
            {
                self.dense.swap(entry1.dense_index, entry2.dense_index);
                self.dense_index_to_sparse_index
                    .swap(entry1.dense_index, entry2.dense_index);

                // swap the references in the sparse array
                self.sparse[index1.sparse_index] = SparseEntry::AliveEntry(AliveSparseEntry {
                    dense_index: entry2.dense_index,
                    epoch: entry1.epoch,
                });
                self.sparse[index2.sparse_index] = SparseEntry::AliveEntry(AliveSparseEntry {
                    dense_index: entry1.dense_index,
                    epoch: entry2.epoch,
                });
            }
            // either there's no element, or there's a newer element the value points to
            _ => {
                panic!("Cannot swap elements that are not alive");
            }
        }
    }

    pub fn get(&self, index: &Index) -> Option<&T> {
        // this can happen only if the index is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(index.sparse_index < self.sparse.len());

        match &self.sparse[index.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == index.epoch => {
                Some(&self.dense[entry.dense_index])
            }
            // either there's no element, or there's a newer element the value points to
            _ => None,
        }
    }

    pub fn get_mut(&mut self, index: &Index) -> Option<&mut T> {
        // this can happen only if the index is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(index.sparse_index < self.sparse.len());

        match &mut self.sparse[index.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == index.epoch => {
                Some(&mut self.dense[entry.dense_index])
            }
            // either there's no element, or there's a newer element the value points to
            _ => None,
        }
    }

    pub fn is_element_alive(&self, index: Index) -> bool {
        if index.sparse_index >= self.sparse.len() {
            return false;
        }

        match &self.sparse[index.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == index.epoch => true,
            _ => false,
        }
    }

    pub fn size(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.dense.iter()
    }

    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> {
        self.dense.iter_mut()
    }

    pub fn key_values(&self) -> impl DoubleEndedIterator<Item = (Index, &T)> {
        self.dense.iter().enumerate().map(move |(i, value)| {
            (
                Index {
                    sparse_index: self.dense_index_to_sparse_index[i],
                    epoch: match &self.sparse[self.dense_index_to_sparse_index[i]] {
                        SparseEntry::AliveEntry(entry) => entry.epoch,
                        SparseEntry::FreeEntry(_) => unreachable!(),
                    },
                },
                value,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // empty sparse set => created => no items
    #[test]
    fn empty_sparse_set_created_no_items() {
        let sparse_set: SparseSet<i32> = SparseSet::new();

        assert_eq!(sparse_set.size(), 0);
        for _ in sparse_set.values() {
            assert!(false);
        }
    }

    // empty sparse set => push item => has one item
    #[test]
    fn empty_sparse_set_push_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();

        let index = sparse_set.push(42);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index), Some(&42));
    }

    // sparse set with one item => mutate the item => the item is changed
    #[test]
    fn sparse_set_with_one_item_mutate_the_item_the_item_is_changed() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        *sparse_set.get_mut(&index).unwrap() = 43;

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index), Some(&43));
    }

    // sparse set with one item => remove_stable item => no items
    #[test]
    fn sparse_set_with_one_item_remove_stable_item_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        sparse_set.remove_stable(&index);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(&index), None);
    }

    // sparse set with one item => remove_swap item => no items
    #[test]
    fn sparse_set_with_one_item_remove_swap_item_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        sparse_set.remove_swap(&index);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(&index), None);
    }

    // sparse set with two items => remove_stable first item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_first_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        sparse_set.remove_stable(&index1);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index1), None);
        assert_eq!(sparse_set.get(&index2), Some(&43));
    }

    // sparse set with two items => remove_swap first item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_swap_first_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        sparse_set.remove_swap(&index1);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index1), None);
        assert_eq!(sparse_set.get(&index2), Some(&43));
    }

    // sparse set with two items => remove_stable second item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_stable_second_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        sparse_set.remove_stable(&index2);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index1), Some(&42));
        assert_eq!(sparse_set.get(&index2), None);
    }

    // sparse set with two items => remove_swap second item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_swap_second_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        sparse_set.remove_swap(&index2);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index1), Some(&42));
        assert_eq!(sparse_set.get(&index2), None);
    }

    // spare set with one item => remove_stable an item and push new item => has one item
    #[test]
    fn sparse_set_with_one_item_remove_an_item_and_push_new_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);
        sparse_set.remove_stable(&index);

        let new_index = sparse_set.push(43);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index), None);
        assert_eq!(sparse_set.get(&new_index), Some(&43));
    }

    // sparse set with one item => remove_swap an item and push new item => has one item
    #[test]
    fn sparse_set_with_one_item_remove_swap_an_item_and_push_new_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);
        sparse_set.remove_swap(&index);

        let new_index = sparse_set.push(43);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index), None);
        assert_eq!(sparse_set.get(&new_index), Some(&43));
    }

    // sparse set with one item => remove_stable item twice => no items
    #[test]
    fn sparse_set_with_one_item_remove_stable_item_twice_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        sparse_set.remove_swap(&index);
        sparse_set.remove_swap(&index);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(&index), None);
    }

    // sparse set with one item => remove_swap item twice => no items
    #[test]
    fn sparse_set_with_one_item_remove_swap_item_twice_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        sparse_set.remove_swap(&index);
        sparse_set.remove_swap(&index);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(&index), None);
    }

    // sparse set with three items => iterate over values => the values are iterated in order
    #[test]
    fn sparse_set_with_three_items_iterate_over_values_the_values_are_iterated_in_order() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        sparse_set.push(42);
        sparse_set.push(43);
        sparse_set.push(44);

        for (i, value) in sparse_set.values().enumerate() {
            if i == 0 {
                assert_eq!(value, &42);
            } else if i == 1 {
                assert_eq!(value, &43);
            } else {
                assert_eq!(value, &44);
            }
        }
    }

    // sparse set with three items => iterate over key-values => the key-values are iterated in order
    #[test]
    fn sparse_set_with_three_items_iterate_over_key_values_the_key_values_are_iterated_in_order() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);
        let index3 = sparse_set.push(44);

        for (i, (index, value)) in sparse_set.key_values().enumerate() {
            if i == 0 {
                assert_eq!(value, &42);
                assert_eq!(index, index1);
            } else if i == 1 {
                assert_eq!(value, &43);
                assert_eq!(index, index2);
            } else {
                assert_eq!(value, &44);
                assert_eq!(index, index3);
            }
        }
    }

    // sparse set with one item => iterate over values and mutate => the value is changed
    #[test]
    fn sparse_set_with_one_item_iterate_over_values_and_mutate_the_value_is_changed() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        for value in sparse_set.values_mut() {
            *value = 43;
        }

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(&index), Some(&43));
    }

    // sparse set with two items => swap the items => the items are swapped in order but not by indexes
    #[test]
    fn sparse_set_with_two_items_swap_the_items_the_items_are_swapped() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        sparse_set.swap(&index1, &index2);

        assert_eq!(sparse_set.size(), 2);
        for (i, value) in sparse_set.values().enumerate() {
            if i == 0 {
                assert_eq!(value, &43);
            } else {
                assert_eq!(value, &42);
            }
        }
        assert_eq!(sparse_set.get(&index1), Some(&42));
        assert_eq!(sparse_set.get(&index2), Some(&43));
    }

    // sparse set with two items => clone the set => cloned set has the same items
    #[test]
    fn sparse_set_with_two_items_clone_the_set_cloned_set_has_the_same_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index1 = sparse_set.push(42);
        let index2 = sparse_set.push(43);

        let cloned_sparse_set = sparse_set.clone();

        assert_eq!(cloned_sparse_set.size(), 2);
        assert_eq!(cloned_sparse_set.get(&index1), Some(&42));
        assert_eq!(cloned_sparse_set.get(&index2), Some(&43));
    }

    // sparse set with one item => check if the item is alive => it is alive
    #[test]
    fn sparse_set_with_one_item_check_if_the_item_is_alive_it_is_alive() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        assert!(sparse_set.is_element_alive(index));
    }

    // sparse set with one item => remove the item and check if the item is alive => it is not alive
    #[test]
    fn sparse_set_with_one_item_remove_the_item_and_check_if_the_item_is_alive_it_is_not_alive() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let index = sparse_set.push(42);

        sparse_set.remove_swap(&index);

        assert!(!sparse_set.is_element_alive(index));
    }
}
