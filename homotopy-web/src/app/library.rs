use yew::prelude::*;

use crate::{
    app::community_library::{self, CommunityPreset},
    app::presets::Preset,
    model::{self, Action},
};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub presets: &'static [Preset],
    pub community_presets: &'static [CommunityPreset],
    pub active_preset: Option<String>,
    pub dispatch: Callback<model::Action>,
    #[prop_or_default]
    pub on_load_preset: Callback<()>,
}

#[function_component]
pub fn LibraryView(props: &Props) -> Html {
    let mut categories: Vec<_> = props.presets.iter().map(|preset| preset.category).collect();
    categories.sort_unstable();
    categories.dedup();
    let lesson = active_lesson(props);

    html! {
        <section class="library-panel">
            <div class="library-panel__content">
                {for categories.into_iter().map(|category| {
                    html! {
                        <section class="library-section">
                            <h3>{category}</h3>
                            {for props.presets.iter().filter(move |preset| preset.category == category).map(|preset| {
                                let id = preset.id.to_owned();
                                let active = props.active_preset.as_deref() == Some(preset.id);
                                let dispatch = props.dispatch.clone();
                                let on_load_preset = props.on_load_preset.clone();
                                html! {
                                    <button
                                        class={classes!("preset-row", active.then_some("preset-row--active"))}
                                        onclick={Callback::from(move |_| {
                                            dispatch.emit(Action::LoadPreset(id.clone()));
                                            on_load_preset.emit(());
                                        })}
                                    >
                                        <span class="preset-row__title">{preset.title}</span>
                                        <span class="preset-row__description">{preset.description}</span>
                                        <span class="preset-row__tags">
                                            {preset.tags.join(" / ")}
                                        </span>
                                    </button>
                                }
                            })}
                        </section>
                    }
                })}
                if !props.community_presets.is_empty() {
                    <section class="library-section">
                        <h3>{"Community"}</h3>
                        {for props.community_presets.iter().map(|preset| {
                            let id = preset.id.clone();
                            let active = props.active_preset.as_deref()
                                == Some(community_library::active_id(&preset.id).as_str());
                            let dispatch = props.dispatch.clone();
                            let on_load_preset = props.on_load_preset.clone();
                            html! {
                                <button
                                    class={classes!("preset-row", active.then_some("preset-row--active"))}
                                    onclick={Callback::from(move |_| {
                                        dispatch.emit(Action::LoadCommunityPreset(id.clone()));
                                        on_load_preset.emit(());
                                    })}
                                >
                                    <span class="preset-row__title">{&preset.title}</span>
                                    <span class="preset-row__description">{&preset.description}</span>
                                    <span class="preset-row__tags">
                                        {preset.tags.join(" / ")}
                                    </span>
                                </button>
                            }
                        })}
                    </section>
                }
            </div>
            <div class="library-panel__lesson">
                <span>{"Didactic note"}</span>
                <p>{lesson}</p>
            </div>
        </section>
    }
}

fn active_lesson(props: &Props) -> String {
    let Some(active) = props.active_preset.as_deref() else {
        return "Choose a preset to load its source and lesson note.".to_owned();
    };

    if let Some(preset) = props.presets.iter().find(|preset| preset.id == active) {
        return preset.lesson.to_owned();
    }

    community_library::parse_active_id(active)
        .and_then(community_library::get)
        .and_then(|preset| preset.didactic.clone())
        .unwrap_or_else(|| "Community presets can include didactic notes after review.".to_owned())
}
