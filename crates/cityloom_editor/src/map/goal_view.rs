//! The guided goal's picker on the map page (#72), at the top of the Notes panel's Checks tab. It offers the change
//! that fits the goal for the selected streets and sends the person to the change picker with it chosen: the review
//! and the apply button of the Selected streets panel stay the only way a street is changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::MapVm;
use crate::shared::bind::Bound;
use crate::shared::goals::Place;

#[component]
pub fn GoalGuide(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || {
        let selected = !vm.with(|v| v.selection().streets()).is_empty();
        let pick: Option<Rc<dyn Fn(&'static str)>> =
            selected.then(|| Rc::new(move |op: &'static str| vm.with(|v| v.set_corridor_op(op))) as Rc<dyn Fn(&'static str)>);
        vm.with(|v| crate::shared::goals_view::goal_guide(v.goal().clone(), Place::Map, v.i18n(), pick))
    }
}
