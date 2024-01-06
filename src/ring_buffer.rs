// Copyright (C) Pavel Grebnev 2023
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

//  |-- head    |-- tail
//  v           v
// [1] [2] [3] [4] [ ]

//      |-- tail    |-- head
//      v           v
// [2] [3] [ ] [ ] [1]

pub struct RingBuffer<T, const SIZE: usize> {
    buffer: [T; SIZE],
    head: usize,
    tail: usize,
    empty: bool,
}

impl<T, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn new(empty_buffer: [T; SIZE]) -> Self {
        Self {
            buffer: empty_buffer,
            head: 0,
            tail: 0,
            empty: true,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.empty {
            self.empty = false;
        } else {
            self.head = (self.head + 1) % SIZE;
            if self.head == self.tail {
                self.tail = (self.tail + 1) % SIZE;
            }
        }
        self.buffer[self.head] = value;
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        return if self.empty {
            self.buffer[..0].iter().chain(self.buffer[..0].iter())
        } else if self.head < self.tail {
            self.buffer[self.tail..]
                .iter()
                .chain(self.buffer[..=self.head].iter())
        } else {
            self.buffer[self.tail..=self.head]
                .iter()
                .chain(self.buffer[..0].iter())
        };
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    #[allow(dead_code)]
    pub fn set_empty(&mut self) {
        self.empty = true;
        self.head = 0;
        self.tail = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ring_buffer_is_empty() {
        let ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);
        assert!(ring_buffer.is_empty());
    }

    #[test]
    fn test_default_ring_buffer_after_push_is_not_empty() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);

        ring_buffer.push(1);

        assert!(!ring_buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_push_items_can_iterate_over_items() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);

        ring_buffer.push(1);
        ring_buffer.push(2);

        let vec: Vec<&i32> = ring_buffer.iter().collect();
        assert_eq!(vec, vec![&1, &2]);
    }

    #[test]
    fn test_full_ring_buffer_can_iterate_over_items() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);
        ring_buffer.push(1);
        ring_buffer.push(2);
        ring_buffer.push(3);

        let vec: Vec<&i32> = ring_buffer.iter().collect();
        assert_eq!(vec, vec![&1, &2, &3]);
    }

    #[test]
    fn test_full_ring_buffer_added_more_items_iterate_only_last_items() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);
        ring_buffer.push(1);
        ring_buffer.push(2);
        ring_buffer.push(3);

        ring_buffer.push(4);
        ring_buffer.push(5);

        let vec: Vec<&i32> = ring_buffer.iter().collect();
        assert_eq!(vec, vec![&3, &4, &5]);
    }

    #[test]
    fn test_full_ring_buffer_adding_items_in_a_cycle_equal_to_iota() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);
        ring_buffer.push(2);
        ring_buffer.push(3);
        ring_buffer.push(4);

        for i in 5..=20 {
            ring_buffer.push(i);
            let result: Vec<i32> = ring_buffer.iter().map(|&x| x).collect();
            let expected: Vec<i32> = (i - 2..=i).collect();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_ring_buffer_set_empty() {
        let mut ring_buffer = RingBuffer::<i32, 3>::new([0; 3]);
        ring_buffer.push(1);
        ring_buffer.push(2);
        ring_buffer.push(3);

        ring_buffer.set_empty();

        assert!(ring_buffer.is_empty());
        assert_eq!(ring_buffer.iter().count(), 0);
    }
}
