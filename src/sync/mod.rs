//! Cloud edits on the app side: the sync document that marks what waits to be sent, the pure rules of merging and
//! batching, and (Task 7) the view-model that runs a sync when a page opens, after a write and when the network
//! comes back.

pub mod hub;
pub mod model;
