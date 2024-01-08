/// This macro creates a unit test that calls the named function, and
/// potentially ignores a `NoAdapter` error.
///
/// # Background
///
/// On GitHub CI, it seems fairly common for the MacOS runners to be configured
/// in such a way that wgpu returns no adapters from a request with the default
/// settings. The default settings appear to imply as high of flexibility as
/// possible, and sometimes the runners succeed in returning an adapter.
///
/// Because of these spurious failures, this macro checks for the environment
/// variable `NO_ADAPTER`. If it is set to a value, a warning will be printed
/// instead of panicking.
#[macro_export]
macro_rules! adapter_required_test {
    ($name:ident) => {
        #[test]
        fn runs() {
            let no_adapter_setting = std::env::var("NO_ADAPTER");
            match ($name(), no_adapter_setting) {
                (Ok(()), _) => {}
                (Err(cushy::window::VirtualRecorderError::NoAdapter), Ok(no_adapter))
                    if !no_adapter.is_empty() =>
                {
                    let prefix = match no_adapter.as_ref() {
                        "github-ci" => "::warning::",
                        _ => "",
                    };
                    println!(
                        "{prefix}Ignoring {}:{}: no graphics adapters available",
                        file!(),
                        stringify!($name)
                    );
                }
                (Err(err), _) => unreachable!("Error testing example: {err}"),
            }
        }
    };
}
