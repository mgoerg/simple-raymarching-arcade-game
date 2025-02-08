

Build / Run:
cargo build
cargo run

Build Wasm:
wasm-pack build --target web

Run Web Server:
python -m http.server 8080
127.0.0.1:8080

Compile and Run:

cmd:
wasm-pack build --dev --target web && python -m http.server 8080

powershell:
(wasm-pack build --dev --target web) -and (python -m http.server 8080)
