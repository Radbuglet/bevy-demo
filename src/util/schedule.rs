use bevy_ecs::schedule::{IntoSystemConfigs, SystemConfigs};

pub fn chain_ambiguous<M>(configs: impl IntoSystemConfigs<M>) -> SystemConfigs {
    // TODO: Only chain ambiguously-ordered systems
    configs.into_configs().chain()
}
