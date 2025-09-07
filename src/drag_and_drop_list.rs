use std::cmp::PartialEq;

const DRAG_START_TIMEOUT_SECONDS: f32 = 0.2;

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

pub(crate) enum DragResult {
    None,
    JustStartedDragging,
    Dragging,
}

pub(crate) enum DropResult {
    None,
    ItemTaken(usize),
    ItemReceived,
    ItemChangedPosition(usize, usize),
}

pub(crate) struct StaticListParameters {
    pub(crate) element_height: f32,
    pub(crate) is_dragging_outside_allowed: bool,
    pub(crate) is_drop_allowed: bool,
    pub(crate) is_reordering_allowed: bool,
}

pub(crate) struct DragAndDropList {
    number_of_elements: usize,
    current_operation: Option<Operation>,
    static_parameters: StaticListParameters,

    bounds: iced::Rectangle,
    scrolling_offset: f32,
}

impl DragAndDropList {
    pub(crate) fn new(number_of_elements: usize, static_parameters: StaticListParameters) -> Self {
        Self {
            number_of_elements,
            current_operation: None,
            static_parameters,

            bounds: iced::Rectangle::new(iced::Point::new(0.0, 0.0), iced::Size::new(0.0, 0.0)),
            scrolling_offset: 0.0,
        }
    }

    pub(crate) fn on_mouse_down(&mut self, position: iced::Point) {
        if self.current_operation.is_some() {
            println!("We already had a drag operation before mouse down, weird");
            self.current_operation = None;
            return;
        }

        if !self.is_mouse_in_bounds(position) {
            return;
        }

        if !self.static_parameters.is_dragging_outside_allowed
            && !self.static_parameters.is_reordering_allowed
        {
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

    pub(crate) fn on_mouse_move(&mut self, position: iced::Point) -> DragResult {
        match self.current_operation.clone() {
            Some(Operation::PreparingForDragging(index, start_time)) => {
                if self.static_parameters.is_dragging_outside_allowed
                    && !self.is_mouse_in_bounds(position)
                {
                    self.current_operation = Some(Operation::DraggingFrom(index));
                    DragResult::JustStartedDragging
                } else {
                    if std::time::Instant::now()
                        .duration_since(start_time)
                        .as_secs_f32()
                        > DRAG_START_TIMEOUT_SECONDS
                    {
                        if self.static_parameters.is_reordering_allowed {
                            self.current_operation = Some(Operation::Reordering(
                                index,
                                self.get_hovered_index(position).unwrap_or(index),
                            ));
                        } else if self.static_parameters.is_dragging_outside_allowed {
                            self.current_operation = Some(Operation::DraggingFrom(index));
                        } else {
                            eprintln!("We can't be in PreparingForDragging with both dragging out and reordering disabled");
                        }
                        DragResult::JustStartedDragging
                    } else if self.static_parameters.is_reordering_allowed {
                        if let Some(hovered_index) = self.get_hovered_index(position) {
                            if hovered_index != index {
                                self.current_operation =
                                    Some(Operation::Reordering(index, hovered_index));
                                DragResult::JustStartedDragging
                            } else {
                                DragResult::None
                            }
                        } else {
                            DragResult::None
                        }
                    } else {
                        DragResult::None
                    }
                }
            }
            Some(Operation::Reordering(index, _hovered_index)) => {
                if self.static_parameters.is_dragging_outside_allowed
                    && !self.is_mouse_in_bounds(position)
                {
                    self.current_operation = Some(Operation::DraggingFrom(index));
                } else {
                    if let Some(hovered_index) = self.get_hovered_index(position) {
                        if hovered_index <= index {
                            self.current_operation =
                                Some(Operation::Reordering(index, hovered_index));
                        } else {
                            self.current_operation =
                                Some(Operation::Reordering(index, hovered_index + 1));
                        }
                    }
                }
                DragResult::Dragging
            }
            Some(Operation::DraggingFrom(index)) => {
                if self.is_mouse_in_bounds(position) && self.static_parameters.is_reordering_allowed
                {
                    if let Some(hovered_index) = self.get_hovered_index(position) {
                        self.current_operation = Some(Operation::Reordering(index, hovered_index));
                    }
                }
                DragResult::Dragging
            }
            Some(Operation::IdleWaitingForDrop) => {
                if self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(Operation::HoveredByDrop);
                }
                DragResult::None
            }
            Some(Operation::HoveredByDrop) => {
                if !self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(Operation::IdleWaitingForDrop);
                }
                DragResult::None
            }
            None => DragResult::None,
        }
    }

    pub(crate) fn on_mouse_up(&mut self, _position: iced::Point) -> DropResult {
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

    // we assume that the compatibility is handled by the caller
    pub(crate) fn started_dragging_somewhere(&mut self) {
        // ignore if it is us dragging from
        if self.static_parameters.is_drop_allowed && self.current_operation.is_none() {
            self.current_operation = Some(Operation::IdleWaitingForDrop);
        }
    }

    pub(crate) fn set_bounds(&mut self, bounds: iced::Rectangle) {
        self.bounds = bounds;
    }

    pub(crate) fn set_scroll_offset(&mut self, offset: f32) {
        self.scrolling_offset = offset;
    }

    pub(crate) fn change_number_of_elements(&mut self, new_number_of_elements: usize) {
        self.number_of_elements = new_number_of_elements;
        self.cancel_operations();
    }

    pub(crate) fn cancel_operations(&mut self) {
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

    pub(crate) fn is_drop_area_hovered(&self) -> bool {
        self.current_operation == Some(Operation::HoveredByDrop)
    }

    fn is_mouse_in_bounds(&self, position: iced::Point) -> bool {
        self.bounds.contains(position)
    }

    fn get_hovered_index(&self, position: iced::Point) -> Option<usize> {
        if self.static_parameters.element_height <= 0.0 {
            return None;
        }

        if position.y < self.bounds.y || position.y > self.bounds.y + self.bounds.height {
            return None;
        }

        if position.y < self.bounds.y {
            return Some(0);
        }

        let index = (position.y + self.scrolling_offset - self.bounds.y)
            / self.static_parameters.element_height;
        let index = index.floor() as usize;

        if index >= self.number_of_elements {
            return None;
        }

        Some(index)
    }
}
