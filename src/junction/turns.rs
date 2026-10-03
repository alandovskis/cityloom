//! The "Turns allowed" table: rows are the street traffic comes from, columns
//! the street it goes to, and a cell allows or bans that turn.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::model::{LEFT, THROUGH};
use crate::junction::read_model::JView;
use crate::junction::watch::Watch;
use crate::junction::vm::JunctionVm;

const COMPASS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/// The compass point a bearing is nearest.
pub fn compass(bearing: i32) -> &'static str {
    COMPASS[((bearing.rem_euclid(360) as f64 / 45.0).round() as usize) % 8]
}

pub fn turn_word(class: u8) -> &'static str {
    match class {
        LEFT => "left",
        THROUGH => "straight on",
        _ => "right",
    }
}

/// How a class of turn is named at the start of a sentence.
pub fn turn_name(class: u8) -> &'static str {
    match class {
        LEFT => "Left",
        THROUGH => "Straight on",
        _ => "Right",
    }
}

/// A turn arrow, pointing up, in a 16 by 16 box: the stem and its branch.
pub fn turn_glyph(class: u8, size: u32) -> impl IntoView {
    let branch = match class {
        LEFT => "M8 9Q8 5 3.5 5M6 3 3.5 5 6 7",
        THROUGH => "M8 9V2M5.5 4.5 8 2l2.5 2.5",
        _ => "M8 9Q8 5 12.5 5M10 3l2.5 2L10 7",
    };
    view! {
        <svg class="turn-ico" viewBox="0 0 16 16" width=size height=size aria-hidden="true" focusable="false">
            <path d="M8 14V9"/>
            <path d=branch/>
        </svg>
    }
}

fn table(v: &JView, toggle: &Rc<dyn Fn(u32, u32, bool)>) -> AnyView {
    let columns = v
        .arms
        .iter()
        .map(|c| {
            let (title, text) = (c.label.clone(), c.label.clone());
            view! {
                <th scope="col" title=title>
                    <span aria-hidden="true">{compass(c.bearing)}</span>
                    <span class="sr-only">{text}</span>
                </th>
            }
        })
        .collect_view();
    let rows = v
        .arms
        .iter()
        .map(|a| {
            let cells = v
                .arms
                .iter()
                .map(|b| -> AnyView {
                    if a.uid == b.uid {
                        return view! { <td class="self" aria-hidden="true">"·"</td> }.into_any();
                    }
                    let Some(m) = v.movements.iter().find(|m| m.from == a.uid && m.to == b.uid) else {
                        return view! { <td class="zero" aria-hidden="true">"–"</td> }.into_any();
                    };
                    let name = format!("{} to {}: {} turn", a.label, b.label, turn_word(m.class));
                    let bad = m.allowed && !m.lane;
                    if let Some(why) = m.blocked {
                        let title = why.to_string();
                        let label = format!("{name}: not possible. {why}");
                        return view! {
                            <td>
                                <button type="button" class="turn locked" disabled title=title aria-label=label>
                                    {turn_glyph(m.class, 16)}
                                </button>
                            </td>
                        }
                        .into_any();
                    }
                    let class = format!("turn{}{}", if m.allowed { " on" } else { "" }, if bad { " bad" } else { "" });
                    let label = format!(
                        "{name}{}",
                        if !m.allowed { ", not allowed" } else if bad { ", allowed, no lane serves it" } else { ", allowed" }
                    );
                    let (from, to, allowed) = (a.uid, b.uid, m.allowed);
                    let toggle = toggle.clone();
                    view! {
                        <td>
                            <button
                                type="button"
                                class=class
                                aria-pressed=allowed.to_string()
                                aria-label=label
                                on:click=move |_| toggle(from, to, allowed)
                            >
                                {turn_glyph(m.class, 16)}
                            </button>
                        </td>
                    }
                    .into_any()
                })
                .collect_view();
            let (title, text) = (a.label.clone(), a.label.clone());
            view! {
                <tr>
                    <th scope="row" title=title>
                        <span class="dirtag">{compass(a.bearing)}</span>
                        <span class="sr-only">{text}</span>
                    </th>
                    {cells}
                </tr>
            }
        })
        .collect_view();
    view! {
        <caption class="sr-only">
            "Turns allowed. Rows are the street traffic comes from, columns the street it goes to."
        </caption>
        <thead>
            <tr>
                <th scope="col"><span class="sr-only">"From"</span></th>
                {columns}
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    }
    .into_any()
}

#[component]
pub fn Turns(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let toggle: Rc<dyn Fn(u32, u32, bool)> = Rc::new(move |from, to, allowed| {
        w.edit(|j| j.set_turn(from, to, !allowed));
    });
    // Not `Send`, which a reactive closure needs: keep it where only this thread
    // can reach it.
    let toggle = StoredValue::new_local(toggle);
    move || {
        let v = w.view();
        toggle.with_value(|t| table(&v, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bearing_names_its_nearest_compass_point() {
        assert_eq!(compass(0), "N");
        assert_eq!(compass(22), "N");
        assert_eq!(compass(23), "NE");
        assert_eq!(compass(90), "E");
        assert_eq!(compass(359), "N");
        assert_eq!(compass(-45), "NW");
    }
}
