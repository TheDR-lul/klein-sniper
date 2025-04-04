fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.compile().expect("Failed to compile resources");
}