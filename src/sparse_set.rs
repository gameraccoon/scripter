// Copyright (C) Pavel Grebnev 2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

/// An implementation of a sparse set, a data structure that stores a set of items and
/// provides a way to efficiently access them by a generated key.
/// It allows for constant time insertions and lookups
/// Good for cache efficiency, doesn't require any hashing,
/// But it is NOT scalable for large data sets
/// The used memory only grows, can free it only by destroying the whole set
#[derive(Clone)]
pub struct SparseSet<T> {
    // has as many values as elements stored in the set
    dense_values: Vec<T>,
    // same size as the dense array, stores keys to the sparse array
    dense_keys: Vec<Key>,
    // stores either index to the value in the dense array or index to the next free sparse slot
    sparse: Vec<SparseEntry>,
    // a "free list" of free entries in the sparse array
    next_free_sparse_entry: usize,
}

/// The key should be stored runtime, it doesn't make sense to serialize it since
/// the SparseSet data is not persistent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
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
            dense_values: Vec::new(),
            dense_keys: Vec::new(),
            sparse: Vec::new(),
            next_free_sparse_entry: usize::MAX,
        }
    }

    pub fn push(&mut self, value: T) -> Key {
        let dense_index = self.dense_values.len();

        // if there are free entries in the sparse array, use one of them
        let key = if self.next_free_sparse_entry != usize::MAX {
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

            Key {
                sparse_index: new_sparse_index,
                epoch: free_sparse_entry.next_epoch,
            }
        } else {
            // extend the sparse array
            self.sparse.push(SparseEntry::AliveEntry(AliveSparseEntry {
                dense_index,
                epoch: 0,
            }));

            Key {
                sparse_index: self.sparse.len() - 1,
                epoch: 0,
            }
        };

        self.dense_values.push(value);
        self.dense_keys.push(key);

        key
    }

    pub fn remove_swap(&mut self, key: Key) -> Option<T> {
        // this can happen only if the key is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(key.sparse_index < self.sparse.len());

        return match self.sparse[key.sparse_index].clone() {
            SparseEntry::AliveEntry(entry) if entry.epoch == key.epoch => {
                let removed_value = self.dense_values.swap_remove(entry.dense_index);
                let swapped_sparse_index = self.dense_keys[self.dense_values.len()].sparse_index;
                self.dense_keys.swap_remove(entry.dense_index);

                if let SparseEntry::AliveEntry(swapped_entry) =
                    &mut self.sparse[swapped_sparse_index]
                {
                    swapped_entry.dense_index = entry.dense_index;
                } else {
                    unreachable!();
                }

                self.sparse[key.sparse_index] = SparseEntry::FreeEntry(FreeSparseEntry {
                    next_free: self.next_free_sparse_entry,
                    next_epoch: usize::wrapping_add(entry.epoch, 1),
                });

                // as long as we have available epochs, we can reuse the sparse index
                if key.epoch < usize::MAX {
                    self.next_free_sparse_entry = key.sparse_index;
                }

                Some(removed_value)
            }
            // the element was already removed (either there's nothing, or a newer element)
            _ => None,
        };
    }

    pub fn remove_stable(&mut self, key: Key) -> Option<T> {
        // this can happen only if the key is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(key.sparse_index < self.sparse.len());

        return match self.sparse[key.sparse_index].clone() {
            SparseEntry::AliveEntry(entry) if entry.epoch == key.epoch => {
                for i in entry.dense_index + 1..self.dense_values.len() {
                    let sparse_index = self.dense_keys[i].sparse_index;
                    if let SparseEntry::AliveEntry(entry) = &mut self.sparse[sparse_index] {
                        entry.dense_index -= 1;
                    } else {
                        unreachable!();
                    }
                }
                let removed_value = self.dense_values.remove(entry.dense_index);
                self.dense_keys.remove(entry.dense_index);

                self.sparse[key.sparse_index] = SparseEntry::FreeEntry(FreeSparseEntry {
                    next_free: self.next_free_sparse_entry,
                    next_epoch: usize::wrapping_add(entry.epoch, 1),
                });

                // as long as we have available epochs, we can reuse the sparse entry
                if key.epoch < usize::MAX {
                    self.next_free_sparse_entry = key.sparse_index;
                }
                Some(removed_value)
            }
            // the element was already removed (either there's nothing, or a newer element)
            _ => None,
        };
    }

    pub fn swap(&mut self, key1: Key, key2: Key) {
        // this can happen only if the key is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(key1.sparse_index < self.sparse.len());
        assert!(key2.sparse_index < self.sparse.len());

        match (
            self.sparse[key1.sparse_index].clone(),
            self.sparse[key2.sparse_index].clone(),
        ) {
            (SparseEntry::AliveEntry(entry1), SparseEntry::AliveEntry(entry2))
                if entry1.epoch == key1.epoch && entry2.epoch == key2.epoch =>
            {
                self.dense_values
                    .swap(entry1.dense_index, entry2.dense_index);
                self.dense_keys.swap(entry1.dense_index, entry2.dense_index);

                // swap the references in the sparse array
                self.sparse[key1.sparse_index] = SparseEntry::AliveEntry(AliveSparseEntry {
                    dense_index: entry2.dense_index,
                    epoch: entry1.epoch,
                });
                self.sparse[key2.sparse_index] = SparseEntry::AliveEntry(AliveSparseEntry {
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

    pub fn get(&self, key: Key) -> Option<&T> {
        // this can happen only if the key is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(key.sparse_index < self.sparse.len());

        match &self.sparse[key.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == key.epoch => {
                Some(&self.dense_values[entry.dense_index])
            }
            // either there's no element, or there's a newer element the value points to
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        // this can happen only if the key is from another SparseSet
        // in this case nothing is guaranteed anymore, we should panic
        assert!(key.sparse_index < self.sparse.len());

        match &mut self.sparse[key.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == key.epoch => {
                Some(&mut self.dense_values[entry.dense_index])
            }
            // either there's no element, or there's a newer element the value points to
            _ => None,
        }
    }

    pub fn is_element_alive(&self, key: Key) -> bool {
        if key.sparse_index >= self.sparse.len() {
            return false;
        }

        match &self.sparse[key.sparse_index] {
            SparseEntry::AliveEntry(entry) if entry.epoch == key.epoch => true,
            _ => false,
        }
    }

    pub fn size(&self) -> usize {
        self.dense_values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense_values.is_empty()
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.dense_values.iter()
    }

    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> {
        self.dense_values.iter_mut()
    }

    pub fn key_values(&self) -> impl DoubleEndedIterator<Item = (&Key, &T)> {
        self.dense_keys.iter().zip(self.dense_values.iter())
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

        let key = sparse_set.push(42);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key), Some(&42));
    }

    // sparse set with one item => mutate the item => the item is changed
    #[test]
    fn sparse_set_with_one_item_mutate_the_item_the_item_is_changed() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        *sparse_set.get_mut(key).unwrap() = 43;

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key), Some(&43));
    }

    // sparse set with one item => remove_stable item => no items
    #[test]
    fn sparse_set_with_one_item_remove_stable_item_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        sparse_set.remove_stable(key);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(key), None);
    }

    // sparse set with one item => remove_swap item => no items
    #[test]
    fn sparse_set_with_one_item_remove_swap_item_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        sparse_set.remove_swap(key);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(key), None);
    }

    // sparse set with two items => remove_stable first item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_first_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        sparse_set.remove_stable(key1);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key1), None);
        assert_eq!(sparse_set.get(key2), Some(&43));
    }

    // sparse set with two items => remove_swap first item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_swap_first_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        sparse_set.remove_swap(key1);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key1), None);
        assert_eq!(sparse_set.get(key2), Some(&43));
    }

    // sparse set with two items => remove_stable second item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_stable_second_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        sparse_set.remove_stable(key2);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key1), Some(&42));
        assert_eq!(sparse_set.get(key2), None);
    }

    // sparse set with two items => remove_swap second item => has one item
    #[test]
    fn sparse_set_with_two_items_remove_swap_second_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        sparse_set.remove_swap(key2);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key1), Some(&42));
        assert_eq!(sparse_set.get(key2), None);
    }

    // spare set with one item => remove_stable an item and push new item => has one item
    #[test]
    fn sparse_set_with_one_item_remove_an_item_and_push_new_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);
        sparse_set.remove_stable(key);

        let new_key = sparse_set.push(43);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key), None);
        assert_eq!(sparse_set.get(new_key), Some(&43));
    }

    // sparse set with one item => remove_swap an item and push new item => has one item
    #[test]
    fn sparse_set_with_one_item_remove_swap_an_item_and_push_new_item_has_one_item() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);
        sparse_set.remove_swap(key);

        let new_key = sparse_set.push(43);

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key), None);
        assert_eq!(sparse_set.get(new_key), Some(&43));
    }

    // sparse set with five items => remove_stable first item => order is not changed
    #[test]
    fn sparse_set_with_five_items_remove_stable_first_item_order_is_not_changed() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);
        let key3 = sparse_set.push(44);
        let key4 = sparse_set.push(45);
        let key5 = sparse_set.push(46);

        sparse_set.remove_stable(key1);

        assert_eq!(sparse_set.size(), 4);
        for (i, value) in sparse_set.values().enumerate() {
            if i == 0 {
                assert_eq!(value, &43);
            } else if i == 1 {
                assert_eq!(value, &44);
            } else if i == 2 {
                assert_eq!(value, &45);
            } else {
                assert_eq!(value, &46);
            }
        }
        assert_eq!(sparse_set.get(key1), None);
        assert_eq!(sparse_set.get(key2), Some(&43));
        assert_eq!(sparse_set.get(key3), Some(&44));
        assert_eq!(sparse_set.get(key4), Some(&45));
        assert_eq!(sparse_set.get(key5), Some(&46));
    }

    // sparse set with one item => remove_stable item twice => no items
    #[test]
    fn sparse_set_with_one_item_remove_stable_item_twice_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        sparse_set.remove_swap(key);
        sparse_set.remove_swap(key);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(key), None);
    }

    // sparse set with one item => remove_swap item twice => no items
    #[test]
    fn sparse_set_with_one_item_remove_swap_item_twice_no_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        sparse_set.remove_swap(key);
        sparse_set.remove_swap(key);

        assert_eq!(sparse_set.size(), 0);
        assert_eq!(sparse_set.get(key), None);
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
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);
        let key3 = sparse_set.push(44);

        for (i, (key, value)) in sparse_set.key_values().enumerate() {
            if i == 0 {
                assert_eq!(value, &42);
                assert_eq!(key, &key1);
            } else if i == 1 {
                assert_eq!(value, &43);
                assert_eq!(key, &key2);
            } else {
                assert_eq!(value, &44);
                assert_eq!(key, &key3);
            }
        }
    }

    // sparse set with one item => iterate over values and mutate => the value is changed
    #[test]
    fn sparse_set_with_one_item_iterate_over_values_and_mutate_the_value_is_changed() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        for value in sparse_set.values_mut() {
            *value = 43;
        }

        assert_eq!(sparse_set.size(), 1);
        assert_eq!(sparse_set.get(key), Some(&43));
    }

    // sparse set with two items => swap the items => the items are swapped in order but not by keys
    #[test]
    fn sparse_set_with_two_items_swap_the_items_the_items_are_swapped_in_order_but_not_by_keys() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        sparse_set.swap(key1, key2);

        assert_eq!(sparse_set.size(), 2);
        for (i, value) in sparse_set.values().enumerate() {
            if i == 0 {
                assert_eq!(value, &43);
            } else {
                assert_eq!(value, &42);
            }
        }
        assert_eq!(sparse_set.get(key1), Some(&42));
        assert_eq!(sparse_set.get(key2), Some(&43));
    }

    // sparse set with two items => clone the set => cloned set has the same items
    #[test]
    fn sparse_set_with_two_items_clone_the_set_cloned_set_has_the_same_items() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key1 = sparse_set.push(42);
        let key2 = sparse_set.push(43);

        let cloned_sparse_set = sparse_set.clone();

        assert_eq!(cloned_sparse_set.size(), 2);
        assert_eq!(cloned_sparse_set.get(key1), Some(&42));
        assert_eq!(cloned_sparse_set.get(key2), Some(&43));
    }

    // sparse set with one item => check if the item is alive => it is alive
    #[test]
    fn sparse_set_with_one_item_check_if_the_item_is_alive_it_is_alive() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        assert!(sparse_set.is_element_alive(key));
    }

    // sparse set with one item => remove the item and check if the item is alive => it is not alive
    #[test]
    fn sparse_set_with_one_item_remove_the_item_and_check_if_the_item_is_alive_it_is_not_alive() {
        let mut sparse_set: SparseSet<i32> = SparseSet::new();
        let key = sparse_set.push(42);

        sparse_set.remove_swap(key);

        assert!(!sparse_set.is_element_alive(key));
    }
}
