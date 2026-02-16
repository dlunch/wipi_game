pub fn step_up(selected: usize) -> usize {
    selected.saturating_sub(1)
}

pub fn step_down(selected: usize, item_count: usize) -> usize {
    if selected + 1 < item_count {
        selected + 1
    } else {
        selected
    }
}
