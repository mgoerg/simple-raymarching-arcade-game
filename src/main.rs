use rust_arcade_game_lib::run;

fn main() {
    print!("Hello, world!");

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run());
    #[cfg(not(target_arch = "wasm32"))]
    pollster::block_on(run());
}