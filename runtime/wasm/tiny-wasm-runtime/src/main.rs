use anyhow::Result;
use tinywasm::execution::{runtime::Runtime, wasi::WasiSnapShotPreview1};

fn main() -> Result<()> {
    let wasi = WasiSnapShotPreview1::new();
    let wasm = include_bytes!("./fixtures/hello_world.wasm");
    let mut runtime = Runtime::instantiate_with_wasi(wasm, wasi)?;
    runtime.call("_start", vec![])?;
    Ok(())
}
