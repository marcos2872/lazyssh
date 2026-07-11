use tachyonfx::{EffectManager, fx, EffectTimer, Interpolation};

/// Gerenciador de efeitos visuais da aplicação (transições, fades, dissolves).
#[derive(Default)]
pub struct AppEffects {
    /// Motor de efeitos do tachyonfx.
    pub manager: EffectManager<()>,
}

impl AppEffects {
    /// Cria um novo gerenciador de efeitos vazio.
    pub fn new() -> Self {
        Self {
            manager: EffectManager::default(),
        }
    }

    /// Adiciona efeito de dissolve ao abrir um modal (200ms).
    pub fn add_modal_open_effect(&mut self) {
        let dissolve = fx::dissolve(200);
        self.manager.add_effect(dissolve);
    }

    /// Adiciona efeito de coalesce ao fechar um modal (150ms, QuadIn).
    pub fn add_modal_close_effect(&mut self) {
        let timer = EffectTimer::from_ms(150, Interpolation::QuadIn);
        let fade = fx::coalesce(timer);
        self.manager.add_effect(fade);
    }

    /// Adiciona efeito de coalesce ao exibir notificação (300ms, BackOut).
    pub fn add_notification_effect(&mut self) {
        let timer = EffectTimer::from_ms(300, Interpolation::BackOut);
        let fade = fx::coalesce(timer);
        self.manager.add_effect(fade);
    }

    /// Adiciona efeito de pulse ao selecionar servidor (150ms, SineInOut).
    pub fn add_server_select_effect(&mut self) {
        let timer = EffectTimer::from_ms(150, Interpolation::SineInOut);
        let pulse = fx::coalesce(timer);
        self.manager.add_effect(pulse);
    }

    /// Adiciona efeito de sweep ao conectar SSH (400ms, QuadOut).
    pub fn add_ssh_connect_effect(&mut self) {
        let timer = EffectTimer::from_ms(400, Interpolation::QuadOut);
        let sweep = fx::coalesce(timer);
        self.manager.add_effect(sweep);
    }

    /// Adiciona efeito de dissolve ao abrir SFTP (250ms).
    pub fn add_sftp_open_effect(&mut self) {
        let dissolve = fx::dissolve(250);
        self.manager.add_effect(dissolve);
    }
}

#[cfg(test)]
impl AppEffects {
    pub(crate) fn set_up_all_effects(&mut self) {
        super::AppEffects::add_modal_open_effect(self);
        super::AppEffects::add_modal_close_effect(self);
        super::AppEffects::add_notification_effect(self);
        super::AppEffects::add_server_select_effect(self);
        super::AppEffects::add_ssh_connect_effect(self);
        super::AppEffects::add_sftp_open_effect(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_default_create_manager() {
        let new_effects = AppEffects::new();
        assert!(!new_effects.manager.is_running());

        let default_effects = AppEffects::default();
        assert!(!default_effects.manager.is_running());
    }

    #[test]
    fn test_modal_open_effect() {
        let mut effects = AppEffects::new();
        effects.add_modal_open_effect();
        assert!(effects.manager.is_running());
    }

    #[test]
    fn test_multiple_effects_stack() {
        let mut effects = AppEffects::new();
        effects.set_up_all_effects();
        assert!(effects.manager.is_running());
    }

    #[test]
    fn test_all_effect_types_are_executed() {
        let mut effects = AppEffects::new();
        effects.set_up_all_effects();
        assert!(effects.manager.is_running());
    }

    #[test]
    fn test_add_server_select_effect() {
        let mut effects = AppEffects::new();
        effects.add_server_select_effect();
        effects.add_ssh_connect_effect();
        effects.add_sftp_open_effect();
        assert!(effects.manager.is_running());
    }

    #[test]
    fn test_modal_close_notification_effects() {
        let mut effects = AppEffects::new();
        effects.add_modal_close_effect();
        effects.add_notification_effect();
        assert!(effects.manager.is_running());
    }

}
