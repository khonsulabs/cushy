use devx_cmd::run;
use khonsu_tools::{anyhow, code_coverage::CodeCoverage};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Args {
    BuildBrowserExample,
    GenerateCodeCoverageReport,
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();
    match args {
        Args::BuildBrowserExample => build_browser_example()?,
        Args::GenerateCodeCoverageReport => CodeCoverage::<CodeCoverageConfig>::execute()?,
    };
    Ok(())
}

fn build_browser_example() -> Result<(), devx_cmd::Error> {
    println!("Executing cargo build");
    run!(
        "cargo",
        "build",
        "--example",
        "basic",
        "--no-default-features",
        "--features",
        "frontend-browser",
        "--target",
        "wasm32-unknown-unknown",
    )?;

    println!("Executing wasm-bindgen (cargo install wasm-bindgen if you don't have this)");
    run!(
        "wasm-bindgen",
        "target/wasm32-unknown-unknown/debug/examples/basic.wasm",
        "--target",
        "web",
        "--out-dir",
        "gooey/examples/browser/pkg/",
        "--remove-producers-section"
    )?;

    println!(
        "Build succeeded. ./examples/browser/index.html can be loaded through any http server \
         that supports wasm."
    );
    println!();
    println!("For example, using `miniserve` (`cargo install miniserve`):");
    println!();
    println!("miniserve gooey/examples/browser/");

    Ok(())
}

struct CodeCoverageConfig;

impl khonsu_tools::code_coverage::Config for CodeCoverageConfig {
    fn ignore_paths() -> Vec<String> {
        vec![String::from("gooey/examples/*")]
    }
}
