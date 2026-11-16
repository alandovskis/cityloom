//! The guided goal's picker on the map page (#72), at the city's level of the details. It offers the change that fits
//! the goal for the selected streets and takes the person to the selected streets' level with it chosen: the review
//! and the apply button there stay the only way a street is changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::MapVm;
use crate::shared::bind::Bound;
use crate::shared::goals::Place;

#[component]
pub fn GoalGuide(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        {move || vm.with(|v| crate::shared::goals_view::goal_picker(v.goal().clone(), v.i18n()))}
        {move || {
            let selected = !vm.with(|v| v.selection().streets()).is_empty();
            let pick: Option<Rc<dyn Fn(&'static str)>> =
                selected.then(|| {
                    Rc::new(move |op: &'static str| {
                        vm.with(|v| v.choose_goal_change(op));
                        super::details::focus_current();
                    }) as Rc<dyn Fn(&'static str)>
                });
            vm.with(|v| crate::shared::goals_view::goal_guidance(v.goal().get(), Place::Map, v.i18n(), pick))
        }}
    }
}
