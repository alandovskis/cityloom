//! The "Turns allowed" table: rows are the street traffic comes from, columns
//! the street it goes to, and a cell allows or bans that turn.

use leptos::prelude::*;

use crate::junction::{Junction, THROUGH, LEFT};
use crate::junction_view::JView;

const COMPASS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/// The compass point a bearing is nearest.
pub fn compass(bearing: i32) -> &'static str {
    COMPASS[((bearing.rem_euclid(360) as f64 / 45.0).round() as usize) % 8]
}

fn turn_word(class: u8) -> &'static str {
    match class {
        LEFT => "left",
        THROUGH => "straight on",
        _ => "right",
    }
}

/// A turn arrow, pointing up, in a 16 by 16 box: the stem and its branch.
fn turn_glyph(class: u8) -> impl IntoView {
    let branch = match class {
        LEFT => "M8 9Q8 5 3.5 5M6 3 3.5 5 6 7",
        THROUGH => "M8 9V2M5.5 4.5 8 2l2.5 2.5",
        _ => "M8 9Q8 5 12.5 5M10 3l2.5 2L10 7",
    };
    view! {
        <svg class="turn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
            <path d="M8 14V9"/>
            <path d=branch/>
        </svg>
    }
}

fn table(v: &JView, toggle: impl Fn(u32, u32, bool) + Copy + 'static) -> AnyView {
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
                                    {turn_glyph(m.class)}
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
                    view! {
                        <td>
                            <button
                                type="button"
                                class=class
                                aria-pressed=allowed.to_string()
                                aria-label=label
                                on:click=move |_| toggle(from, to, allowed)
                            >
                                {turn_glyph(m.class)}
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
pub fn Turns() -> impl IntoView {
    let model = StoredValue::new_local(Junction::new(0));
    let view = RwSignal::new_local(model.with_value(|j| j.view()));
    let toggle = move |from: u32, to: u32, allowed: bool| {
        model.update_value(|j| {
            j.set_turn(from, to, !allowed);
        });
        view.set(model.with_value(|j| j.view()));
    };
    move || view.with(|v| table(v, toggle))
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
