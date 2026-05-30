use yew::prelude::*;

use crate::{
    app::presets::Preset,
    model::{self, Action},
};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub presets: &'static [Preset],
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
            </div>
            <div class="library-panel__lesson">
                <span>{"Step 2 of 5"}</span>
                <p>{"Edit a schema once, then instantiate it with different cells to create concrete diagrams."}</p>
            </div>
        </section>
    }
}
