use agent_settings::AgentSettings;
use fs::Fs;
use gpui::{Entity, FocusHandle, SharedString};
use picker::popover_menu::PickerPopoverMenu;

use crate::ModelUsageContext;
use assistant_context_editor::language_model_selector::{
    LanguageModelSelector, ToggleModelSelector, language_model_selector,
};
use language_model::{ConfiguredModel, LanguageModelRegistry};
use settings::{Settings, update_settings_file};
use std::sync::Arc;
use ui::{ButtonLike, PopoverMenuHandle, Tooltip, prelude::*};

pub struct AgentModelSelector {
    selector: Entity<LanguageModelSelector>,
    menu_handle: PopoverMenuHandle<LanguageModelSelector>,
    focus_handle: FocusHandle,
}

impl AgentModelSelector {
    pub(crate) fn new(
        fs: Arc<dyn Fs>,
        menu_handle: PopoverMenuHandle<LanguageModelSelector>,
        focus_handle: FocusHandle,
        model_usage_context: ModelUsageContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            selector: cx.new(move |cx| {
                let fs = fs.clone();
                language_model_selector(
                    {
                        let model_context = model_usage_context.clone();
                        move |cx| model_context.configured_model(cx)
                    },
                    move |model, cx| {
                        let provider = model.provider_id().0.to_string();
                        let model_id = model.id().0.to_string();
                        match &model_usage_context {
                            ModelUsageContext::Thread(thread) => {
                                thread.update(cx, |thread, cx| {
                                    let registry = LanguageModelRegistry::read_global(cx);
                                    if let Some(provider) = registry.provider(&model.provider_id())
                                    {
                                        thread.set_configured_model(
                                            Some(ConfiguredModel {
                                                provider,
                                                model: model.clone(),
                                            }),
                                            cx,
                                        );
                                    }
                                });
                                update_settings_file::<AgentSettings>(
                                    fs.clone(),
                                    cx,
                                    move |settings, _cx| {
                                        settings.set_model(model.clone());
                                    },
                                );
                            }
                            ModelUsageContext::InlineAssistant => {
                                update_settings_file::<AgentSettings>(
                                    fs.clone(),
                                    cx,
                                    move |settings, _cx| {
                                        settings.set_inline_assistant_model(
                                            provider.clone(),
                                            model_id.clone(),
                                        );
                                    },
                                );
                            }
                        }
                    },
                    window,
                    cx,
                )
            }),
            menu_handle,
            focus_handle,
        }
    }

    pub fn toggle(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_handle.toggle(window, cx);
    }
}

impl Render for AgentModelSelector {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();

        let model = self.selector.read(cx).delegate.active_model(cx);
        let model_name = model
            .as_ref()
            .map(|model| model.model.name().0)
            .unwrap_or_else(|| SharedString::from("No model selected"));
        let show_icons = AgentSettings::get_global(cx).always_show_provider_icon;
        let provider_icon = show_icons
            .then(|| {
                model.as_ref().and_then(|m| {
                    LanguageModelRegistry::read_global(cx)
                        .provider(&m.model.provider_id())
                        .map(|p| p.icon())
                })
            })
            .flatten();
        PickerPopoverMenu::new(
            self.selector.clone(),
            ButtonLike::new("active-model").child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .when_some(provider_icon, |el, icon| {
                        el.child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
                    })
                    .child(
                        Label::new(model_name)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        Icon::new(IconName::ChevronDown)
                            .size(IconSize::XSmall)
                            .color(Color::Muted),
                    ),
            ),
            move |window, cx| {
                Tooltip::for_action_in(
                    "Change Model",
                    &ToggleModelSelector,
                    &focus_handle,
                    window,
                    cx,
                )
            },
            gpui::Corner::BottomRight,
            cx,
        )
        .with_handle(self.menu_handle.clone())
        .render(window, cx)
    }
}
