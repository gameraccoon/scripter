use std::cmp::PartialEq;

#[derive(Debug, Clone, PartialEq)]
enum Operation {
    // we just pressed the mouse button, we don't know yet if it is a drag or a click
    PreparingForDragging(usize, std::time::Instant),
    // the element with the first index is being dragged from this list
    // the position before the second index is being hovered
    // we can transition from Reordering to DraggingFrom if we move the dragged element
    // outside the list and this is allowed
    Reordering(usize, usize),
    // the element with the given index is being dragged from the list
    // to the area outside the list
    // we can transition from DraggingFrom to Reordering if we move the dragged element
    // if we move the dragged element back to the list and this is allowed
    DraggingFrom(usize),
    // we are showing the drop area (not hovered)
    // we can transition from WaitingForDrop to HoveredByDrop if we hover the drop area
    IdleWaitingForDrop,
    // we are hovering over the drop area
    // we can transition from HoveredByDrop to WaitingForDrop if we move the mouse away
    HoveredByDrop,
}

pub(crate) enum DropResult {
    None,
    ItemTaken(usize),
    ItemReceived,
    ItemChangedPosition(usize, usize),
}

pub(crate) struct StaticListParameters {
    element_height: f32,
    is_dragging_outside_allowed: bool,
    is_drop_allowed: bool,
    is_reordering_allowed: bool,
}

pub(crate) struct DragAndDropList {
    number_of_elements: usize,
    current_operation: Option<Operation>,
    static_parameters: StaticListParameters,
}

impl Default for DragAndDropList {
    fn default() -> Self {
        Self {
            number_of_elements: 0,
            current_operation: None,
            static_parameters: StaticListParameters {
                element_height: 0.0,
                is_dragging_outside_allowed: false,
                is_drop_allowed: false,
                is_reordering_allowed: false,
            },
        }
    }
}

impl DragAndDropList {
    pub(crate) fn new(
        number_of_elements: usize,
        element_height: f32,
        static_parameters: StaticListParameters,
    ) -> Self {
        Self {
            number_of_elements,
            current_operation: None,
            static_parameters,
        }
    }

    pub(crate) fn on_mouse_down(&mut self, position: iced::Point) {
        if self.current_operation.is_some() {
            println!("We already had an drag operation before mouse down, weird");
            self.current_operation = None;
            return;
        }

        if !self.is_mouse_in_bounds(position) {
            return;
        }

        let hovered_index = self.get_hovered_index(position);

        if let Some(hovered_index) = hovered_index {
            self.current_operation = Some(Operation::PreparingForDragging(
                hovered_index,
                std::time::Instant::now(),
            ));
        }
    }

    pub(crate) fn on_mouse_move(&mut self, position: iced::Point) {
        match &mut self.current_operation {
            Some(Operation::PreparingForDragging(index, start_time)) => {}
            Some(Operation::Reordering(index, hovered_index)) => {}
            Some(Operation::DraggingFrom(index)) => {}
            Some(Operation::IdleWaitingForDrop) => {
                if self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(Operation::HoveredByDrop);
                }
            }
            Some(Operation::HoveredByDrop) => {
                if !self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(Operation::IdleWaitingForDrop);
                }
            }
            None => {}
        }
    }

    pub(crate) fn on_mouse_up(&mut self, position: iced::Point) -> DropResult {
        if !self.is_mouse_in_bounds(position) {
            return DropResult::None;
        }

        // clean and return
        match self.current_operation.take() {
            Some(Operation::PreparingForDragging(_index, _start_time)) => DropResult::None,
            Some(Operation::Reordering(index, hovered_index)) => {
                DropResult::ItemChangedPosition(index, hovered_index)
            }
            Some(Operation::DraggingFrom(index)) => DropResult::ItemTaken(index),
            Some(Operation::IdleWaitingForDrop) => DropResult::None,
            Some(Operation::HoveredByDrop) => DropResult::ItemReceived,
            None => DropResult::None,
        }
    }

    pub(crate) fn started_dragging_elsewhere(&mut self) {
        if self.static_parameters.is_drop_allowed && self.current_operation.is_none() {
            self.current_operation = Some(Operation::IdleWaitingForDrop);
        }
    }

    pub(crate) fn resize(&mut self, new_number_of_elements: usize) {
        self.number_of_elements = new_number_of_elements;
        self.current_operation = None;
    }

    pub(crate) fn get_dragged_element_index(&self) -> Option<usize> {
        match &self.current_operation {
            Some(Operation::DraggingFrom(index)) => Some(*index),
            Some(Operation::Reordering(index, _)) => Some(*index),
            _ => None,
        }
    }

    pub(crate) fn get_reordering_target_index(&self) -> Option<usize> {
        match &self.current_operation {
            Some(Operation::Reordering(_, target_index)) => Some(*target_index),
            _ => None,
        }
    }

    pub(crate) fn should_show_drop_area(&self) -> bool {
        self.current_operation == Some(Operation::IdleWaitingForDrop)
            || self.current_operation == Some(Operation::HoveredByDrop)
    }

    pub(crate) fn is_drop_area_hovered(&self, position: iced::Point) -> bool {
        self.current_operation == Some(Operation::HoveredByDrop)
    }

    fn is_mouse_in_bounds(&self, position: iced::Point) -> bool {
        false
    }

    fn get_hovered_index(&self, position: iced::Point) -> Option<usize> {
        None
    }
}
