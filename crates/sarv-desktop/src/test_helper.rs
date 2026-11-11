#[cfg(test)]
pub fn make_demo_state() -> crate::app::State {
    let engine = crate::data::Engine::demo();
    crate::app::State::with_engine(engine, true)
}
