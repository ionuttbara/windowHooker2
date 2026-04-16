extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", "1.0.29565.1000");
    res.set("ProductVersion", "1.0.29565.1000");
    res.set("CompanyName", "Gallery Inc");
    res.set("FileDescription", "IBB-Hooker");
    res.set("ProductName", "IBB-Hooker");
    
    // Add the icon to the executable
    res.set_icon("../hooker.ico");
    
    // Compile the resources (required to actually embed the metadata and icon!)
    res.compile().expect("Failed to compile Windows resources!");
}