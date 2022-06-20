mod input;
mod system;

pub use self::{input::*, system::*};

use phantom_dependencies::{gilrs::Gilrs, winit::window::Window};
use phantom_gui::Gui;
use phantom_render::Renderer;

pub struct Resources<'a> {
    pub window: &'a mut Window,
    pub gilrs: &'a mut Gilrs,
    pub renderer: &'a mut Box<dyn Renderer>,
    pub gui: &'a mut Gui,
    pub input: &'a mut Input,
    pub system: &'a mut System,
}

impl<'a> Resources<'a> {
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.window.set_cursor_visible(visible)
    }
}
