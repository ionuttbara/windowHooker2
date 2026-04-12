extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", "1.0.29565.1000");
    res.set("ProductVersion", "1.0.29565.1000");
    res.set("CompanyName", "Gallery Inc");
    res.set("FileDescription", "IBB-Hooker");
    res.set("ProductName", "IBB-Hooker");
}