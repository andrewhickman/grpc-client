mod address;
mod controller;
mod request;
mod response;

use std::{collections::BTreeMap, sync::Arc};

use druid::{
    widget::{Flex, Label},
    Data, Lens, Widget, WidgetExt as _, WidgetId,
};
use iter_set::Inclusion;

use self::controller::TabController;
use crate::{
    protobuf::ProtobufMethod,
    theme,
    widget::{TabId, TabLabelState, Tabs, TabsData, TabsDataChange},
};

#[derive(Debug, Default, Clone, Data, Lens)]
pub(in crate::app) struct State {
    tabs: Arc<BTreeMap<TabId, TabState>>,
    selected: Option<TabId>,
}

#[derive(Debug, Clone, Data, Eq, PartialEq)]
enum RequestState {
    NotStarted,
    Active,
}

#[derive(Debug, Clone, Data, Lens)]
pub struct TabState {
    method: ProtobufMethod,
    #[lens(ignore)]
    address: address::AddressState,
    request: request::State,
    response: response::State,
    #[lens(ignore)]
    request_state: RequestState,
}

pub(in crate::app) fn build() -> Box<dyn Widget<State>> {
    Tabs::new(build_body).boxed()
}

fn build_body() -> impl Widget<TabState> {
    let id = WidgetId::next();
    Flex::column()
        .must_fill_main_axis(true)
        .with_child(address::build().lens(TabState::address_lens()))
        .with_spacer(theme::GUTTER_SIZE)
        .with_child(Label::new("Request").align_left())
        .with_spacer(theme::GUTTER_SIZE)
        .with_flex_child(request::build().lens(TabState::request), 0.5)
        .with_spacer(theme::GUTTER_SIZE)
        .with_child(Label::new("Response").align_left())
        .with_spacer(theme::GUTTER_SIZE)
        .with_flex_child(response::build().lens(TabState::response), 0.5)
        .padding(theme::GUTTER_SIZE)
        .controller(TabController::new(id))
        .with_id(id)
}

impl State {
    pub fn select_or_create_tab(&mut self, method: ProtobufMethod) {
        if self
            .with_selected(|_, tab_data| tab_data.method.same(&method))
            .unwrap_or(false)
        {
            return;
        }

        for (&id, tab) in self.tabs.iter() {
            if tab.method.same(&method) {
                self.selected = Some(id);
                return;
            }
        }

        self.create_tab(method)
    }

    pub fn create_tab(&mut self, method: ProtobufMethod) {
        let id = TabId::next();
        self.selected = Some(id);
        Arc::make_mut(&mut self.tabs).insert(
            id,
            TabState {
                request: request::State::new(method.clone()),
                response: response::State::default(),
                address: address::AddressState::default(),
                request_state: RequestState::NotStarted,
                method,
            },
        );
    }

    pub fn selected_method(&self) -> Option<ProtobufMethod> {
        self.with_selected(|_, tab_data| tab_data.method.clone())
    }
}

impl TabState {
    pub fn can_send(&self) -> bool {
        self.request_state == RequestState::NotStarted
            && self.address.is_valid()
            && self.request.is_valid()
    }

    pub(in crate::app) fn address_lens() -> impl Lens<TabState, address::State> {
        struct AddressLens;

        impl Lens<TabState, address::State> for AddressLens {
            fn with<V, F: FnOnce(&address::State) -> V>(&self, data: &TabState, f: F) -> V {
                f(&address::State::new(data.address.clone(), data.can_send()))
            }

            fn with_mut<V, F: FnOnce(&mut address::State) -> V>(
                &self,
                data: &mut TabState,
                f: F,
            ) -> V {
                let mut address_data = address::State::new(data.address.clone(), data.can_send());
                let result = f(&mut address_data);

                debug_assert_eq!(data.can_send(), address_data.can_send());
                if !data.address.same(address_data.address_state()) {
                    data.address = address_data.into_address_state();
                }

                result
            }
        }

        AddressLens
    }
}

impl TabsData for State {
    type Item = TabState;

    fn selected(&self) -> Option<TabId> {
        self.selected
    }

    fn with_selected<V>(&self, f: impl FnOnce(TabId, &Self::Item) -> V) -> Option<V> {
        self.selected
            .map(|tab_id| f(tab_id, self.tabs.get(&tab_id).unwrap()))
    }

    fn with_selected_mut<V>(&mut self, f: impl FnOnce(TabId, &mut Self::Item) -> V) -> Option<V> {
        self.selected.map(|tab_id| {
            let tab_data = self.tabs.get(&tab_id).unwrap();
            let mut new_tab_data = tab_data.clone();
            let result = f(tab_id, &mut new_tab_data);

            if !tab_data.same(&new_tab_data) {
                *Arc::make_mut(&mut self.tabs).get_mut(&tab_id).unwrap() = new_tab_data;
            }

            result
        })
    }

    fn for_each(&self, mut f: impl FnMut(TabId, &Self::Item)) {
        for (&tab_id, tab_data) in self.tabs.iter() {
            f(tab_id, tab_data)
        }
    }

    fn for_each_mut(&mut self, mut f: impl FnMut(TabId, &mut Self::Item)) {
        let mut new_tabs = self.tabs.clone();
        for (&tab_id, tab_data) in self.tabs.iter() {
            let mut new_tab_data = tab_data.clone();
            f(tab_id, &mut new_tab_data);

            if !tab_data.same(&new_tab_data) {
                Arc::make_mut(&mut new_tabs).insert(tab_id, new_tab_data);
            }
        }
        self.tabs = new_tabs;
    }

    fn for_each_changed(&self, old: &Self, mut f: impl FnMut(TabId, TabsDataChange<Self::Item>)) {
        for inclusion in
            iter_set::classify_by_key(old.tabs.iter(), self.tabs.iter(), |(&tab_id, _)| tab_id)
        {
            match inclusion {
                Inclusion::Left((&tab_id, _)) => {
                    f(tab_id, TabsDataChange::Removed);
                }
                Inclusion::Both(_, (&tab_id, tab_data)) => {
                    f(tab_id, TabsDataChange::Changed(tab_data));
                }
                Inclusion::Right((&tab_id, _)) => {
                    f(tab_id, TabsDataChange::Added);
                }
            }
        }
    }

    fn for_each_label(&self, mut f: impl FnMut(TabId, &TabLabelState)) {
        for (&tab_id, tab_data) in self.tabs.iter() {
            let selected = self.selected == Some(tab_id);
            let label_data = TabLabelState::new(tab_data.method.name().clone(), selected);

            f(tab_id, &label_data);
        }
    }

    fn for_each_label_mut(&mut self, mut f: impl FnMut(TabId, &mut TabLabelState)) {
        for (&tab_id, tab_data) in self.tabs.iter() {
            let selected = self.selected == Some(tab_id);
            let mut label_data = TabLabelState::new(tab_data.method.name().clone(), selected);

            f(tab_id, &mut label_data);

            if selected != label_data.selected() {
                self.selected = if label_data.selected() {
                    Some(tab_id)
                } else {
                    None
                }
            }
        }
    }

    fn for_each_label_changed(
        &self,
        old: &Self,
        mut f: impl FnMut(TabId, TabsDataChange<TabLabelState>),
    ) {
        for inclusion in
            iter_set::classify_by_key(old.tabs.iter(), self.tabs.iter(), |(&tab_id, _)| tab_id)
        {
            match inclusion {
                Inclusion::Left((&tab_id, _)) => {
                    f(tab_id, TabsDataChange::Removed);
                }
                Inclusion::Both(_, (&tab_id, tab_data)) => {
                    let selected = self.selected == Some(tab_id);
                    let label_data = TabLabelState::new(tab_data.method.name().clone(), selected);

                    f(tab_id, TabsDataChange::Changed(&label_data));
                }
                Inclusion::Right((&tab_id, _)) => {
                    f(tab_id, TabsDataChange::Added);
                }
            }
        }
    }

    fn remove(&mut self, id: TabId) {
        Arc::make_mut(&mut self.tabs).remove(&id);

        self.selected = self
            .tabs
            .range(id..)
            .next()
            .or_else(|| self.tabs.range(..id).next_back())
            .map(|(&tab_id, _)| tab_id);
    }
}
