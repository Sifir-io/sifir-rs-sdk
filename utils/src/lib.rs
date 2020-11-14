pub enum CallBackResult {
    Success(String),
    Error(String),
}
pub trait CallBack {
    fn on_state_changed(&self, result: CallBackResult);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
