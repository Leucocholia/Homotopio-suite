use gloo::timers::callback::Timeout;
use homotopy_dsl::{Diagnostic, Severity, SymbolInfo};
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use yew::prelude::*;

use crate::model::{self, Action};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Vec<SymbolInfo>,
    pub dispatch: Callback<model::Action>,
}

pub enum Msg {
    Changed(String),
    Apply,
    RetryEditor,
}

pub struct SourcePanel {
    host: NodeRef,
    editor: Option<JsValue>,
    on_change: Option<Closure<dyn Fn(String)>>,
    retry: Option<Timeout>,
    mount_attempts: u8,
    last_source: String,
    last_diagnostics: Vec<Diagnostic>,
}

impl Component for SourcePanel {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            host: NodeRef::default(),
            editor: None,
            on_change: None,
            retry: None,
            mount_attempts: 0,
            last_source: ctx.props().source.clone(),
            last_diagnostics: Vec::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        if self.editor.is_some()
            && self
                .host
                .cast::<web_sys::Element>()
                .map_or(false, |element| element.child_element_count() == 0)
        {
            if let Some(editor) = self.editor.take() {
                let _ = bridge_call("destroy", &[editor]);
            }
            self.on_change = None;
        }

        if self.editor.is_none() && !self.mount_editor(ctx) {
            self.schedule_retry(ctx);
            return;
        }

        if self.last_source != ctx.props().source {
            if let Some(editor) = &self.editor {
                let _ = bridge_call(
                    "setValue",
                    &[editor.clone(), JsValue::from_str(&ctx.props().source)],
                );
            }
            self.last_source = ctx.props().source.clone();
        }

        if self.last_diagnostics != ctx.props().diagnostics {
            if let Some(editor) = &self.editor {
                if let Ok(value) = serde_wasm_bindgen::to_value(&ctx.props().diagnostics) {
                    let _ = bridge_call("setDiagnostics", &[editor.clone(), value]);
                }
            }
            self.last_diagnostics = ctx.props().diagnostics.clone();
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Changed(source) => {
                ctx.props().dispatch.emit(Action::SetSource(source));
                false
            }
            Msg::Apply => {
                ctx.props().dispatch.emit(Action::ApplySource);
                false
            }
            Msg::RetryEditor => {
                self.retry = None;
                true
            }
        }
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let errors = ctx
            .props()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        let warnings = ctx.props().diagnostics.len().saturating_sub(errors);

        html! {
            <aside class="source-panel">
                <div class="panel-header">
                    <span class="panel-title">{"Source"}</span>
                    <button class="source-panel__apply" onclick={ctx.link().callback(|_| Msg::Apply)}>{"Apply"}</button>
                </div>
                <div class="source-panel__editor" ref={self.host.clone()} />
                <div class="diagnostics">
                    <div class="diagnostics__header">
                        <span>{"Diagnostics"}</span>
                        <span>{format!("{errors} errors / {warnings} warnings")}</span>
                    </div>
                    if ctx.props().diagnostics.is_empty() {
                        <p class="diagnostics__empty">{"No diagnostics."}</p>
                    } else {
                        {for ctx.props().diagnostics.iter().map(view_diagnostic)}
                    }
                </div>
                <div class="symbol-list">
                    <h3>{"Symbols"}</h3>
                    {for ctx.props().symbols.iter().map(|symbol| html! {
                        <div class="symbol-list__row">
                            <span>{&symbol.name}</span>
                            <span>{format!("{}D", symbol.dimension)}</span>
                        </div>
                    })}
                </div>
            </aside>
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.retry = None;
        if let Some(editor) = self.editor.take() {
            let _ = bridge_call("destroy", &[editor]);
        }
    }
}

impl SourcePanel {
    fn mount_editor(&mut self, ctx: &Context<Self>) -> bool {
        let Some(element) = self.host.cast::<web_sys::Element>() else {
            return false;
        };
        let callback = ctx.link().callback(Msg::Changed);
        let on_change = Closure::wrap(Box::new(move |value: String| {
            callback.emit(value);
        }) as Box<dyn Fn(String)>);
        let on_change_value: JsValue = on_change
            .as_ref()
            .unchecked_ref::<Function>()
            .clone()
            .into();

        let Some(editor) = bridge_call(
            "create",
            &[
                element.into(),
                JsValue::from_str(&ctx.props().source),
                on_change_value,
            ],
        ) else {
            return false;
        };

        self.editor = Some(editor);
        self.on_change = Some(on_change);
        self.mount_attempts = 0;
        true
    }

    fn schedule_retry(&mut self, ctx: &Context<Self>) {
        if self.retry.is_some() || self.mount_attempts >= 40 {
            return;
        }

        self.mount_attempts += 1;
        let link = ctx.link().clone();
        self.retry = Some(Timeout::new(50, move || {
            link.send_message(Msg::RetryEditor);
        }));
    }
}

fn view_diagnostic(diagnostic: &Diagnostic) -> Html {
    let class = match diagnostic.severity {
        Severity::Error => "diagnostics__item diagnostics__item--error",
        Severity::Warning => "diagnostics__item diagnostics__item--warning",
    };
    html! {
        <div class={class}>
            <span>{format!("{}..{}", diagnostic.span.start, diagnostic.span.end)}</span>
            <p>{&diagnostic.message}</p>
        </div>
    }
}

fn bridge_call(name: &str, args: &[JsValue]) -> Option<JsValue> {
    let window: JsValue = web_sys::window()?.into();
    let bridge = Reflect::get(&window, &JsValue::from_str("HomotopyEditor")).ok()?;
    let function = Reflect::get(&bridge, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    let array = Array::new();
    for arg in args {
        array.push(arg);
    }
    function.apply(&bridge, &array).ok()
}
