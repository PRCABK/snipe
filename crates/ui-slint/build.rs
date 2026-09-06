fn main() {
    let config = slint_build::CompilerConfiguration::new();
    slint_build::compile_with_config("../../ui/overlay.slint", config.clone()).unwrap();
    slint_build::compile_with_config("../../ui/pin.slint", config.clone()).unwrap();
    slint_build::compile_with_config("../../ui/ai-result.slint", config.clone()).unwrap();
    slint_build::compile_with_config("../../ui/settings.slint", config).unwrap();
}
