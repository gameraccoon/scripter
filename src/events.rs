// Copyright (C) Pavel Grebnev 2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::main_window::MainWindow;
use crate::main_window_utils::{
    cancel_all_drag_and_drop_operations, get_current_script_list_drag_and_drop,
    set_execution_lists_scroll_offset, update_drag_and_drop_area_bounds,
};

pub(crate) fn on_script_list_pane_content_height_decreased(app: &mut MainWindow) {
    // there is a bug in Iced, that when the content pane height decreases to the point when
    // the scroll bar is no longer visible, the on_scroll event is not triggered
    // which results in inability to detect that case and update the scroll offset

    // what we do instead is always reset the scroll offset, and then let it update down the line
    // when the on_scroll event is triggered
    get_current_script_list_drag_and_drop(app).set_scroll_offset(0.0);
}

pub(crate) fn on_execution_pane_content_height_decreased(app: &mut MainWindow) {
    // there is a bug in Iced, that when the content pane height decreases to the point when
    // the scroll bar is no longer visible, the on_scroll event is not triggered
    // which results in inability to detect that case and update the scroll offset

    // what we do instead is always reset the scroll offset, and then let it update down the line
    // when the on_scroll event is triggered
    set_execution_lists_scroll_offset(app, 0.0);
}

pub(crate) fn on_window_resized(app: &mut MainWindow, size: iced::Size) {
    if !app.window_state.has_maximized_pane {
        app.window_state.full_window_size = size;
    }
    update_drag_and_drop_area_bounds(app);
    cancel_all_drag_and_drop_operations(app);
}
