use std::cmp::PartialEq;

const DRAG_START_TIMEOUT_SECONDS: f32 = 0.2;

#[derive(Debug, Clone, PartialEq)]
enum DragOperation {
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
}

pub(crate) enum DragResult {
    None,
    JustStartedDragging(usize),
    Dragging,
}

pub(crate) enum DropResult {
    None,
    ItemTaken,
    ItemChangedPosition(usize, usize),
}

pub(crate) struct StaticDragAreaParameters {
    pub(crate) element_height: f32,
    pub(crate) is_dragging_outside_allowed: bool,
    pub(crate) is_reordering_allowed: bool,
    pub(crate) are_bounds_dynamic: bool,
}

pub(crate) struct DragAndDropList {
    number_of_elements: usize,
    current_operation: Option<DragOperation>,
    static_parameters: StaticDragAreaParameters,

    bounds: iced::Rectangle,
    scrolling_offset: f32,
}

impl DragAndDropList {
    pub(crate) fn new(
        number_of_elements: usize,
        static_parameters: StaticDragAreaParameters,
    ) -> Self {
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

        if !self.is_position_in_bounds(position) {
            return;
        }

        if !self.static_parameters.is_dragging_outside_allowed
            && !self.static_parameters.is_reordering_allowed
        {
            return;
        }

        let hovered_index = self.get_hovered_index(position);

        if let Some(hovered_index) = hovered_index {
            self.current_operation = Some(DragOperation::PreparingForDragging(
                hovered_index,
                std::time::Instant::now(),
            ));
        }
    }

    pub(crate) fn on_mouse_move(&mut self, position: iced::Point) -> DragResult {
        match self.current_operation.clone() {
            Some(DragOperation::PreparingForDragging(index, start_time)) => {
                if self.static_parameters.is_dragging_outside_allowed
                    && !self.is_position_in_bounds(position)
                {
                    self.current_operation = Some(DragOperation::DraggingFrom(index));
                    DragResult::JustStartedDragging(index)
                } else {
                    if std::time::Instant::now()
                        .duration_since(start_time)
                        .as_secs_f32()
                        > DRAG_START_TIMEOUT_SECONDS
                    {
                        if self.static_parameters.is_reordering_allowed {
                            self.current_operation = Some(DragOperation::Reordering(
                                index,
                                self.get_hovered_index(position).unwrap_or(index),
                            ));
                        } else if self.static_parameters.is_dragging_outside_allowed {
                            self.current_operation = Some(DragOperation::DraggingFrom(index));
                        } else {
                            eprintln!("We can't be in PreparingForDragging with both dragging out and reordering disabled");
                        }
                        DragResult::JustStartedDragging(index)
                    } else if self.static_parameters.is_reordering_allowed {
                        if let Some(hovered_index) = self.get_hovered_index(position) {
                            if hovered_index != index {
                                self.current_operation =
                                    Some(DragOperation::Reordering(index, hovered_index));
                                DragResult::JustStartedDragging(index)
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
            Some(DragOperation::Reordering(index, _hovered_index)) => {
                if self.static_parameters.is_dragging_outside_allowed
                    && !self.is_position_in_bounds(position)
                {
                    self.current_operation = Some(DragOperation::DraggingFrom(index));
                } else {
                    if let Some(hovered_index) = self.get_hovered_index(position) {
                        if hovered_index <= index {
                            self.current_operation =
                                Some(DragOperation::Reordering(index, hovered_index));
                        } else {
                            self.current_operation =
                                Some(DragOperation::Reordering(index, hovered_index + 1));
                        }
                    }
                }
                DragResult::Dragging
            }
            Some(DragOperation::DraggingFrom(index)) => {
                if self.is_position_in_bounds(position)
                    && self.static_parameters.is_reordering_allowed
                {
                    if let Some(hovered_index) = self.get_hovered_index(position) {
                        self.current_operation =
                            Some(DragOperation::Reordering(index, hovered_index));
                    }
                }
                DragResult::Dragging
            }
            None => DragResult::None,
        }
    }

    pub(crate) fn on_mouse_up(&mut self, _position: iced::Point) -> DropResult {
        // clean and return
        match self.current_operation.take() {
            Some(DragOperation::PreparingForDragging(_index, _start_time)) => DropResult::None,
            Some(DragOperation::Reordering(index, hovered_index)) => {
                DropResult::ItemChangedPosition(index, hovered_index)
            }
            Some(DragOperation::DraggingFrom(_index)) => DropResult::ItemTaken,
            None => DropResult::None,
        }
    }

    pub(crate) fn set_bounds(&mut self, bounds: iced::Rectangle) {
        self.bounds = bounds;
        if self.static_parameters.are_bounds_dynamic {
            self.bounds.height =
                self.static_parameters.element_height * self.number_of_elements as f32;
        }
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
            Some(DragOperation::DraggingFrom(index)) => Some(*index),
            Some(DragOperation::Reordering(index, _)) => Some(*index),
            _ => None,
        }
    }

    pub(crate) fn get_reordering_target_index(&self) -> Option<usize> {
        match &self.current_operation {
            Some(DragOperation::Reordering(_, target_index)) => Some(*target_index),
            _ => None,
        }
    }

    fn is_position_in_bounds(&self, position: iced::Point) -> bool {
        if self.static_parameters.are_bounds_dynamic {
            self.bounds
                .contains(position + iced::Vector::new(0.0, self.scrolling_offset))
        } else {
            self.bounds.contains(position)
        }
    }

    fn get_hovered_index(&self, position: iced::Point) -> Option<usize> {
        if self.static_parameters.element_height <= 0.0 {
            return None;
        }

        if !self.is_position_in_bounds(position) {
            return None;
        }

        let index = (position.y + self.scrolling_offset - self.bounds.y)
            / self.static_parameters.element_height;

        if index < 0.0 {
            return None;
        }

        let index = index.floor() as usize;

        if index >= self.number_of_elements {
            return None;
        }

        Some(index)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum DropOperation {
    // we are showing the drop area (not hovered)
    // we can transition from WaitingForDrop to HoveredByDrop if we hover the drop area
    IdleWaitingForDrop,
    // we are hovering over the drop area
    // we can transition from HoveredByDrop to WaitingForDrop if we move the mouse away
    HoveredByItem,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DropAreaState {
    Inactive,
    VisibleIdle,
    HoveredByItem,
}

pub(crate) struct DropArea {
    current_operation: Option<DropOperation>,
    bounds: iced::Rectangle,
    scrolling_offset: f32,
}

impl DropArea {
    pub(crate) fn new() -> Self {
        Self {
            current_operation: None,
            bounds: iced::Rectangle::new(iced::Point::new(0.0, 0.0), iced::Size::new(0.0, 0.0)),
            scrolling_offset: 0.0,
        }
    }

    pub(crate) fn on_mouse_move(&mut self, position: iced::Point) {
        match self.current_operation.clone() {
            Some(DropOperation::IdleWaitingForDrop) => {
                if self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(DropOperation::HoveredByItem);
                }
            }
            Some(DropOperation::HoveredByItem) => {
                if !self.is_mouse_in_bounds(position) {
                    self.current_operation = Some(DropOperation::IdleWaitingForDrop);
                }
            }
            None => {}
        }
    }

    pub(crate) fn on_mouse_up(&mut self, _position: iced::Point) -> bool {
        // clean and return
        match self.current_operation.take() {
            Some(DropOperation::IdleWaitingForDrop) => false,
            Some(DropOperation::HoveredByItem) => true,
            None => false,
        }
    }

    pub(crate) fn on_started_dragging_compatible_element(&mut self) {
        self.current_operation = Some(DropOperation::IdleWaitingForDrop);
    }

    pub(crate) fn set_bounds(&mut self, bounds: iced::Rectangle) {
        self.bounds = bounds;
    }

    pub(crate) fn set_scroll_offset(&mut self, offset: f32) {
        self.scrolling_offset = offset;
    }

    pub(crate) fn cancel_operations(&mut self) {
        self.current_operation = None;
    }

    pub(crate) fn get_drop_area_state(&self) -> DropAreaState {
        match &self.current_operation {
            Some(DropOperation::IdleWaitingForDrop) => DropAreaState::VisibleIdle,
            Some(DropOperation::HoveredByItem) => DropAreaState::HoveredByItem,
            None => DropAreaState::Inactive,
        }
    }

    pub(crate) fn get_bounds_scrolled(&self) -> iced::Rectangle {
        self.bounds - iced::Vector::new(0.0, self.scrolling_offset)
    }

    fn is_mouse_in_bounds(&self, position: iced::Point) -> bool {
        self.bounds
            .contains(position + iced::Vector::new(0.0, self.scrolling_offset))
    }
}
