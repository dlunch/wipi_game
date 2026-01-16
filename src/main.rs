#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

use wipi::{app::App, kernel::exit, println, wipi_main};

#[derive(Default)]
pub struct HelloWorldApp;

impl HelloWorldApp {
    pub fn new() -> Self {
        println!("Hello, world! {}", 2024);
        exit(0);

        Self {}
    }
}

impl App for HelloWorldApp {}

#[wipi_main]
pub fn main() -> HelloWorldApp {
    HelloWorldApp::new()
}
