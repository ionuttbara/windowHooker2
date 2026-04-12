extern crate winres;
use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch_str = if target_arch == "x86" { "x86" } else { "x64" };
    let desc = format!("IBB-HookerModule ({})", arch_str);

    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", "13.0.29560.1000");
    res.set("ProductVersion", "13.0.29560.1000");
    res.set("CompanyName", "Gallery Inc");
    res.set("FileDescription", &desc);
    res.set("ProductName", &desc);
    res.compile().unwrap();
}