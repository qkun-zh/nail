#![cfg(feature = "end_to_end")]

#[path = "../../../test/end_to_end/smtp_sink.rs"]
mod smtp_sink;

#[path = "../../../test/end_to_end/context.rs"]
mod context;

#[path = "../../../test/end_to_end/flows.rs"]
mod flows;
