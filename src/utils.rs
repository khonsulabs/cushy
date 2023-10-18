use kludgine::app::winit::keyboard::ModifiersState;

pub trait ModifiersExt {
    fn primary(&self) -> bool;
}

impl ModifiersExt for ModifiersState {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn primary(&self) -> bool {
        self.super_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn primary(&self) -> bool {
        self.control_key()
    }
}
