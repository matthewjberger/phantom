use phantom::{
    app::{run, AppConfig, MouseOrbit, Resources, State, Transition},
    dependencies::{
        anyhow::{Context, Result},
        egui::{global_dark_light_mode_switch, menu, SidePanel, TopBottomPanel},
        gilrs::Event as GilrsEvent,
        log,
        winit::event::{ElementState, Event, KeyboardInput, MouseButton},
    },
    world::{load_gltf, World},
};

pub struct Editor {
    camera: MouseOrbit,
    world: World,
}

impl Editor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            camera: MouseOrbit::default(),
            world: World::new()?,
        })
    }
}

impl State for Editor {
    fn label(&self) -> String {
        "Phantom Editor - Main".to_string()
    }

    fn world(&mut self) -> Result<Option<&mut World>> {
        Ok(Some(&mut self.world))
    }

    fn on_start(&mut self, resources: &mut Resources) -> Result<()> {
        log::info!("Starting the Phantom editor");
        load_gltf("assets/models/arena.glb", &mut self.world)?;
        resources.renderer.load_world(&self.world)
    }

    fn on_stop(&mut self, _resources: &mut Resources) -> Result<()> {
        log::info!("Stopping the Phantom editor");
        Ok(())
    }

    fn on_pause(&mut self, _resources: &mut Resources) -> Result<()> {
        log::info!("Editor paused");
        Ok(())
    }

    fn on_resume(&mut self, _resources: &mut Resources) -> Result<()> {
        log::info!("Editor unpaused");
        Ok(())
    }

    fn update(&mut self, resources: &mut Resources) -> Result<Transition> {
        if self.world.active_camera_is_main()? {
            let camera_entity = self.world.active_camera()?;
            self.camera
                .update(&mut self.world, resources, camera_entity)?;
        }
        Ok(Transition::None)
    }

    fn update_gui(&mut self, resources: &mut Resources) -> Result<Transition> {
        let ctx = &resources.gui.context();

        TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ctx, |ui| {
                menu::bar(ui, |ui| {
                    global_dark_light_mode_switch(ui);
                });
            });

        SidePanel::left("scene_explorer")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Scene Explorer");
                ui.allocate_space(ui.available_size());
            });

        SidePanel::right("inspector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.allocate_space(ui.available_size());
            });

        TopBottomPanel::bottom("console")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Assets");
                ui.allocate_space(ui.available_size());
            });

        Ok(Transition::None)
    }

    fn on_gamepad_event(
        &mut self,
        _resources: &mut Resources,
        event: GilrsEvent,
    ) -> Result<Transition> {
        let GilrsEvent { id, time, event } = event;
        log::trace!("{:?} New gamepad event from {}: {:?}", time, id, event);
        Ok(Transition::None)
    }

    fn on_file_dropped(
        &mut self,
        _resources: &mut Resources,
        path: &std::path::Path,
    ) -> Result<Transition> {
        log::info!(
            "File dropped: {}",
            path.as_os_str()
                .to_str()
                .context("Failed to convert path!")?
        );
        Ok(Transition::None)
    }

    fn on_mouse(
        &mut self,
        _resources: &mut Resources,
        button: &MouseButton,
        button_state: &ElementState,
    ) -> Result<Transition> {
        log::trace!("Mouse event: {:#?} {:#?}", button, button_state);
        Ok(Transition::None)
    }

    fn on_key(&mut self, _resources: &mut Resources, input: KeyboardInput) -> Result<Transition> {
        log::trace!("Key event received: {:#?}", input);
        Ok(Transition::None)
    }

    fn on_event(&mut self, _resources: &mut Resources, _event: &Event<()>) -> Result<Transition> {
        Ok(Transition::None)
    }
}

fn main() -> Result<()> {
    Ok(run(
        Editor::new()?,
        AppConfig {
            icon: Some("assets/icons/phantom.png".to_string()),
            ..Default::default()
        },
    )?)
}
