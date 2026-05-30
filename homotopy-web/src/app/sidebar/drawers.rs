use yew::{callback::Callback, prelude::*};

use super::{DrawerViewSize, Sidebar, SidebarButton, SidebarDrawer, SidebarMsg, SidebarProps};
#[cfg(any(debug_assertions, feature = "show_debug_panel"))]
use crate::app::debug::DebugView;
use crate::{
    app::{
        account::AccountView, image_export::ImageExportView, library::LibraryView, presets,
        project::ProjectView, settings::SettingsView, signature::SignatureView,
        source::SourcePanel, stash::StashView,
    },
    components::Visible,
    model::{
        self,
        proof::{Action, SignatureEdit},
        Proof,
    },
};

macro_rules! declare_sidebar_drawers {
    ($(
        $(#[cfg($cfg:meta)])?
        $name:ident {
            $title:literal,
            $class:literal,
            $icon:literal,
            $body:expr,
            $(min_width: $min_width:expr,)?
            $(top_icon: $top_icon:expr,
              top_icon_action: $action:expr,)?
        }
    )*) => {
        #[allow(unused)]
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum NavDrawer {
            $(
                $(#[cfg($cfg)])*
                $name
            ),*
        }

        impl NavDrawer {
            #[allow(clippy::let_underscore_untyped)]
            pub(super) fn view(
                self,
                model_dispatch: &Callback<model::Action>,
                sidebar_dispatch: &Callback<SidebarMsg>,
                props: &SidebarProps,
                initial_width: i32,
                drawer_view_size: DrawerViewSize,
            ) -> Html {
                match self {
                    $(
                        $(#[cfg($cfg)])?
                        NavDrawer::$name => {
                            let body = $body;
                            html! {
                                <SidebarDrawer
                                    title={$title}
                                    class={$class}
                                    initial_width={initial_width}
                                    $(min_width={$min_width})?
                                    model_dispatch={model_dispatch}
                                    sidebar_dispatch={sidebar_dispatch}
                                    $(icon={$top_icon})?
                                    $(on_click={
                                        let action = $action;
                                        action(&props.proof)
                                    })?
                                >
                                    {body(model_dispatch, sidebar_dispatch, props, drawer_view_size)}
                                </SidebarDrawer>
                            }
                        }
                    ),*
                }
            }
        }

        impl Sidebar {
            pub(super) fn nav(&self, ctx: &Context<Self>) -> Html {
                html! {
                    <nav class="sidebar__nav">
                    $({{
                        $(#[cfg($cfg)])?
                        html! {
                            <SidebarButton
                                label={$title}
                                icon={$icon}
                                action={SidebarMsg::Toggle(Some(NavDrawer::$name))}
                                shortcut={None}
                                dispatch={ctx.link().callback(|x| x)}
                                visibility={Visible}
                            />
                        }

                        $(
                            #[cfg(not($cfg))]
                            html! {}
                        )?
                    }})*
                    </nav>
                }
            }
        }
    }
}

declare_sidebar_drawers! {
    DRAWER_LOGIN {
        "Account",
        "account",
        "account_circle",
        |dispatch, _, props: &SidebarProps, _| html! {
            <AccountView
                dispatch={dispatch}
                proof={props.proof.clone()}
                remote_project_metadata={props.remote_project_metadata.clone()}
            />
        },
        min_width: 250,
    }

    DRAWER_PROJECT {
        "Project",
        "project",
        "info",
        |dispatch, _, props: &SidebarProps, _| html! {
            <ProjectView
                dispatch={dispatch}
                metadata={props.proof.metadata.clone()}
            />
        },
        min_width: 250,
    }

    DRAWER_LIBRARY {
        "Library",
        "library",
        "menu_book",
        |dispatch, sidebar_dispatch: &Callback<SidebarMsg>, props: &SidebarProps, _| html! {
            <LibraryView
                presets={presets::PRESETS}
                active_preset={props.active_preset.clone()}
                dispatch={dispatch}
                on_load_preset={sidebar_dispatch.reform(|_| SidebarMsg::Toggle(Some(NavDrawer::DRAWER_SOURCE)))}
            />
        },
        min_width: 280,
    }

    DRAWER_SIGNATURE {
        "Signature",
        "signature",
        "list",
        |dispatch, _, props: &SidebarProps, drawer_view_size: DrawerViewSize| html! {
            <SignatureView
                signature={props.proof.signature.clone()}
                dispatch={dispatch}
                drawer_view_size={drawer_view_size}
            />
        },
        min_width: 250,
        top_icon: "create_new_folder",
        top_icon_action: |proof: &Proof| model::Action::Proof(Action::EditSignature(SignatureEdit::NewFolder(proof.signature.as_tree().root()))),
    }

    DRAWER_SOURCE {
        "Source",
        "source",
        "code",
        |dispatch: &Callback<model::Action>, _, props: &SidebarProps, _| html! {
            <div class="source-drawer">
                <div class="source-drawer__actions">
                    <button class="source-panel__apply" onclick={dispatch.reform(|_| model::Action::ApplySource)}>{"Apply"}</button>
                </div>
                <SourcePanel
                    source={props.source.clone()}
                    diagnostics={props.source_diagnostics.clone()}
                    symbols={props.source_symbols.clone()}
                    dispatch={dispatch}
                />
            </div>
        },
        min_width: 320,
    }

    DRAWER_STASH {
        "Stash",
        "stash",
        "bookmarks",
        |dispatch, _, props: &SidebarProps, _| html! {
            <StashView
                stash={props.proof.stash.clone()}
                dispatch={dispatch}
                signature={props.proof.signature.clone()}
            />
        },
        min_width: 250,
    }

    DRAWER_IMAGE_EXPORT {
        "Image export",
        "ImageExport",
        "output",
        |dispatch, _, props: &SidebarProps, _| match props.proof.workspace.as_ref() {
            None => html! {
                <p>{"There is nothing to export."}</p>
            },
            Some(ws) => html!{
                <ImageExportView
                    dispatch={dispatch}
                    view_dimension={ws.view.dimension()}
                    dimension={ws.visible_dimension()}
                />
            },
        },
        min_width: 250,
    }

    DRAWER_SETTINGS {
        "Settings",
        "settings",
        "settings",
        |_, _, _, _| html! {
            <SettingsView />
        },
        min_width: 250,
    }

    #[cfg(any(debug_assertions, feature = "show_debug_panel"))]
    DRAWER_DEBUG {
        "Debug",
        "debug",
        "bug_report",
        |dispatch, _, props: &SidebarProps, _| html! {
            <DebugView proof={props.proof.clone()} dispatch={dispatch} />
        },
        min_width: 250,
    }
}
