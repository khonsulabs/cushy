use devx_cmd::run;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Args {
    BuildBrowserExample,
}

fn main() -> Result<(), devx_cmd::Error> {
    let args = Args::from_args();
    let Args::BuildBrowserExample = args;
    build_browser_example()?;
    Ok(())
}

fn build_browser_example() -> Result<(), devx_cmd::Error> {
    println!("Executing cargo build");
    run!(
        "cargo",
        "build",
        "--example",
        "browser",
        "--no-default-features",
        "--features",
        "frontend-browser",
        "--target",
        "wasm32-unknown-unknown",
    )?;

    println!("Executing wasm-bindgen (cargo install wasm-bindgen if you don't have this)");
    run!(
        "wasm-bindgen",
        "target/wasm32-unknown-unknown/debug/examples/browser.wasm",
        "--target",
        "web",
        "--out-dir",
        "gooey/examples/browser/pkg/",
        "--remove-producers-section"
    )?;

    println!("Build succeeded. ./examples/browser/index.html can be loaded through any http server that supports wasm.");
    println!();
    println!("For example, using `miniserve` (`cargo install miniserve`):");
    println!();
    println!("miniserve gooey/examples/browser/");

    Ok(())
}
