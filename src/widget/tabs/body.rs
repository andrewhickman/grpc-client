use std::collections::BTreeMap;

use druid::{widget::prelude::*, Widget, WidgetPod};

use super::{TabId, TabsData, TabsDataChange};
use crate::theme;

pub struct TabsBody<T: TabsData, F, W> {
    children: BTreeMap<TabId, WidgetPod<T::Item, W>>,
    make_child: F,
}

impl<T: TabsData, F, W> TabsBody<T, F, W> {
    pub fn new(make_child: F) -> Self {
        TabsBody {
            children: BTreeMap::new(),
            make_child,
        }
    }
}

impl<T, F, W> Widget<T> for TabsBody<T, F, W>
where
    T: TabsData,
    F: FnMut() -> W,
    W: Widget<T::Item>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if hidden_should_receive_event(event) {
            self.for_each_mut(data, |_, tab, tab_data| {
                tab.event(ctx, event, tab_data, env)
            })
        } else {
            data.with_selected_mut(|id, tab_data| {
                self.children
                    .get_mut(&id)
                    .unwrap()
                    .event(ctx, event, tab_data, env)
            });
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if hidden_should_receive_lifecycle(event) {
            self.for_each(data, |_, tab, tab_data| {
                tab.lifecycle(ctx, event, tab_data, env)
            })
        } else {
            data.with_selected(|id, tab_data| {
                self.children
                    .get_mut(&id)
                    .unwrap()
                    .lifecycle(ctx, event, tab_data, env)
            });
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        data.for_each_changed(old_data, |id, change| match change {
            TabsDataChange::Added => {
                self.children
                    .insert(id, WidgetPod::new((self.make_child)()));
                ctx.children_changed();
            }
            TabsDataChange::Changed(label_data) => {
                if let Some(label) = self.children.get_mut(&id) {
                    label.update(ctx, label_data, env);
                } else {
                    log::error!("TabBody out of sync with data");
                }
            }
            TabsDataChange::Removed => {
                self.children.remove(&id);
                ctx.children_changed();
            }
        });

        if old_data.selected() != data.selected() && data.selected().is_some() {
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        data.with_selected(|id, tab_data| {
            let body = self.children.get_mut(&id).unwrap();
            let size = body.layout(ctx, bc, tab_data, env);
            body.set_layout_rect(ctx, tab_data, env, size.to_rect());
            size
        })
        .unwrap_or(bc.min())
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        data.with_selected(|id, tab_data| {
            let bounds = ctx.size().to_rect();
            ctx.fill(bounds, &env.get(theme::TAB_BACKGROUND));

            self.children
                .get_mut(&id)
                .unwrap()
                .paint_raw(ctx, tab_data, env)
        });
    }
}

impl<T, G, W> TabsBody<T, G, W>
where
    T: TabsData,
{
    fn for_each<F>(&mut self, data: &T, mut f: F)
    where
        F: FnMut(TabId, &mut WidgetPod<T::Item, W>, &T::Item),
    {
        let mut children = self.children.iter_mut();
        data.for_each(|tab_id, tab_data| match children.next() {
            Some((&tab_id2, tab)) if tab_id == tab_id2 => f(tab_id, tab, tab_data),
            _ => log::error!("TabBody out of sync with data"),
        })
    }

    fn for_each_mut<F>(&mut self, data: &mut T, mut f: F)
    where
        F: FnMut(TabId, &mut WidgetPod<T::Item, W>, &mut T::Item),
    {
        let mut children = self.children.iter_mut();
        data.for_each_mut(|tab_id, tab_data| match children.next() {
            Some((&tab_id2, tab)) if tab_id == tab_id2 => f(tab_id, tab, tab_data),
            _ => log::error!("TabBody out of sync with data"),
        })
    }
}

fn hidden_should_receive_event(evt: &Event) -> bool {
    match evt {
        Event::WindowConnected
        | Event::WindowSize(_)
        | Event::Timer(_)
        | Event::AnimFrame(_)
        | Event::Command(_)
        | Event::Internal(_) => true,
        Event::MouseDown(_)
        | Event::MouseUp(_)
        | Event::MouseMove(_)
        | Event::Wheel(_)
        | Event::KeyDown(_)
        | Event::KeyUp(_)
        | Event::Paste(_)
        | Event::Zoom(_) => false,
    }
}

fn hidden_should_receive_lifecycle(lc: &LifeCycle) -> bool {
    match lc {
        LifeCycle::WidgetAdded | LifeCycle::Internal(_) => true,
        LifeCycle::Size(_) | LifeCycle::HotChanged(_) | LifeCycle::FocusChanged(_) => false,
    }
}