//! The view-models: what a page means, with no page in it. A view-model owns
//! the model, turns it into properties a view can bind to as they are (words,
//! flags, lists, all ready to show), and takes commands from the view. It reaches
//! the browser only through the ports in `ports`, so it is tested on the host.

pub mod binding;
pub mod camera;
pub mod city_store;
pub mod junction;
pub mod junction_text;
pub mod map;
pub mod map_gestures;
pub mod plan_frame;
pub mod shell;
pub mod street;
pub mod street_text;
