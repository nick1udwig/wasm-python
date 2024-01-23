use anyhow::Result;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

fn main() -> Result<()> {
    // Define the WASI functions globally on the `Config`.
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    let dir = cap_std::fs::Dir::open_ambient_dir(
        "module/target/wasm32-wasi/debug",
        cap_std::ambient_authority(),
    ).unwrap();

    let wasi_stdin = ReadPipe::from("import sys; from pprint import pprint as pp; pp(sys.path); pp(sys.platform)");
    let wasi_stdout = WritePipe::new_in_memory();

    // Create a WASI context and put it in a Store; all instances in the store
    // share this context. `WasiCtxBuilder` provides a number of ways to
    // configure what the target program will have access to.
    let wasi = WasiCtxBuilder::new()
        .preopened_dir(dir, "/")?
        .stdin(Box::new(wasi_stdin.clone()))
        .stdout(Box::new(wasi_stdout.clone()))
        .inherit_args()?
        .build();
    let mut store = Store::new(&engine, wasi);

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, "python-3.12.0.wasm")?;
    linker.module(&mut store, "", &module)?;
    linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ())?;

    drop(store);
    let contents: Vec<u8> = wasi_stdout
        .try_into_inner()
        .expect("sole remaining reference to WritePipe")
        .into_inner();
    let contents = String::from_utf8(contents)?;
    println!("contents of stdout: {}", contents);

    Ok(())
}
