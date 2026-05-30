use homotopy_dsl::{Diagnostic, Severity, SymbolInfo};
use yew::prelude::*;

use crate::{
    app::{library::LibraryView, presets::Preset, source::SourcePanel},
    model::{self, Action},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WorkbenchTab {
    Source,
    Library,
}

pub enum Msg {
    Select(WorkbenchTab),
    Apply,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub presets: &'static [Preset],
    pub active_preset: Option<String>,
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Vec<SymbolInfo>,
    pub dispatch: Callback<model::Action>,
}

pub struct WorkbenchPanel {
    active: WorkbenchTab,
}

impl Component for WorkbenchPanel {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            active: WorkbenchTab::Source,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Select(tab) => {
                if self.active == tab {
                    false
                } else {
                    self.active = tab;
                    true
                }
            }
            Msg::Apply => {
                ctx.props().dispatch.emit(Action::ApplySource);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let errors = ctx
            .props()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();

        html! {
            <aside class="workbench-panel">
                <div class="workbench-panel__header">
                    <div class="workbench-tabs" role="tablist">
                        {self.tab_button(ctx, WorkbenchTab::Source, "Source", errors)}
                        {self.tab_button(ctx, WorkbenchTab::Library, "Library", 0)}
                    </div>
                    if self.active == WorkbenchTab::Source {
                        <button class="source-panel__apply" onclick={ctx.link().callback(|_| Msg::Apply)}>{"Apply"}</button>
                    }
                </div>
                <div class="workbench-panel__content">
                    {match self.active {
                        WorkbenchTab::Source => html! {
                            <SourcePanel
                                source={ctx.props().source.clone()}
                                diagnostics={ctx.props().diagnostics.clone()}
                                symbols={ctx.props().symbols.clone()}
                                dispatch={ctx.props().dispatch.clone()}
                            />
                        },
                        WorkbenchTab::Library => html! {
                            <LibraryView
                                presets={ctx.props().presets}
                                active_preset={ctx.props().active_preset.clone()}
                                dispatch={ctx.props().dispatch.clone()}
                                on_load_preset={ctx.link().callback(|_| Msg::Select(WorkbenchTab::Source))}
                            />
                        },
                    }}
                </div>
            </aside>
        }
    }
}

impl WorkbenchPanel {
    fn tab_button(
        &self,
        ctx: &Context<Self>,
        tab: WorkbenchTab,
        label: &'static str,
        count: usize,
    ) -> Html {
        let selected = self.active == tab;
        html! {
            <button
                class={classes!("workbench-tab", selected.then_some("workbench-tab--active"))}
                role="tab"
                aria-selected={selected.to_string()}
                onclick={ctx.link().callback(move |_| Msg::Select(tab))}
            >
                <span>{label}</span>
                if count > 0 {
                    <span class="workbench-tab__badge">{count}</span>
                }
            </button>
        }
    }
}
