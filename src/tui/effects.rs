use std::time::Duration;
use tachyonfx::{EffectManager, fx, EffectTimer, Interpolation};

#[derive(Default)]
pub struct AppEffects {
    pub manager: EffectManager<()>,
}

impl AppEffects {
    pub fn new() -> Self {
        Self {
            manager: EffectManager::default(),
        }
    }

    pub fn add_modal_open_effect(&mut self) {
        let dissolve = fx::dissolve(200);
        self.manager.add_effect(dissolve);
    }

    pub fn add_modal_close_effect(&mut self) {
        let timer = EffectTimer::from_ms(150, Interpolation::QuadIn);
        let fade = fx::coalesce(timer);
        self.manager.add_effect(fade);
    }

    pub fn add_notification_effect(&mut self) {
        let timer = EffectTimer::from_ms(300, Interpolation::BackOut);
        let fade = fx::coalesce(timer);
        self.manager.add_effect(fade);
    }

    pub fn add_server_select_effect(&mut self) {
        let timer = EffectTimer::from_ms(150, Interpolation::SineInOut);
        let pulse = fx::coalesce(timer);
        self.manager.add_effect(pulse);
    }

    pub fn add_ssh_connect_effect(&mut self) {
        let timer = EffectTimer::from_ms(400, Interpolation::QuadOut);
        let sweep = fx::coalesce(timer);
        self.manager.add_effect(sweep);
    }

    pub fn add_sftp_open_effect(&mut self) {
        let dissolve = fx::dissolve(250);
        self.manager.add_effect(dissolve);
    }
}
