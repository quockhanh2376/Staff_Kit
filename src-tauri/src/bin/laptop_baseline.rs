// One-time laptop baseline: `laptop_baseline <db> <xlsx> [--apply]`. Run with the app closed.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(staff_kit_lib::laptop_baseline_cli(&args));
}
