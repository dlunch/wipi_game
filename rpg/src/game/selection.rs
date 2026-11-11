pub fn step_up(selected: usize) -> usize {
    if selected > 0 { selected - 1 } else { 0 }
}

pub fn step_down(selected: usize, item_count: usize) -> usize {
    if selected + 1 < item_count {
        selected + 1
    } else {
        selected
    }
}
