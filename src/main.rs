use anyhow::Result;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    let wasi_stdin = ReadPipe::from("import sys; from pprint import pprint as pp; pp(sys.path); pp(sys.platform)");
    let wasi_stdout = WritePipe::new_in_memory();

    {
        // Define the WASI functions globally on the `Config`.
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        // uncomment to bring in non-stdlib libs (except those containing C code)
        // let dir = cap_std::fs::Dir::open_ambient_dir(
        //     "venv",
        //     cap_std::ambient_authority(),
        // ).unwrap();

        // Create a WASI context and put it in a Store; all instances in the store
        // share this context. `WasiCtxBuilder` provides a number of ways to
        // configure what the target program will have access to.
        let wasi = WasiCtxBuilder::new()
            // uncomment to bring in non-stdlib libs (except those containing C code)
            // .preopened_dir(dir, "/venv")?
            // .env("PYTHONPATH", "/venv/lib/python3.12/site-packages")?
            .stdin(Box::new(wasi_stdin.clone()))
            .stdout(Box::new(wasi_stdout.clone()))
            .inherit_args()?
            .build();
        let mut store = Store::new(&engine, wasi);

        // Instantiate our module with the imports we've created, and run it.
        let module = Module::from_file(&engine, "python-3.12.0.wasm")?;
        linker.module_async(&mut store, "", &module).await?;
        linker
            .get_default(&mut store, "")?
            .typed::<(), ()>(&store)?
            .call_async(&mut store, ()).await?;
    }

    let contents: Vec<u8> = wasi_stdout
        .try_into_inner()
        .expect("sole remaining reference to WritePipe")
        .into_inner();
    let contents = String::from_utf8(contents)?;
    println!("contents of stdout: {}", contents);

    Ok(())
}
