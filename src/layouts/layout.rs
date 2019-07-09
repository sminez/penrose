pub trait Layout {
    fn focused_client(&self) -> Option<&Window>;
    fn remove_focused(&mut self) -> Option<Window>;
    fn insert_client(&mut self, win: Window);

    fn focus_next(&mut self);
    fn focus_prev(&mut self);

    fn inc_master(&mut self, px: i32);
    fn dec_master(&mut self, px: i32);
}
