fn main() {
    glib_build_tools::compile_resources(
        "resources/data/icons",
        "resources/data/icons/icons.gresource.xml",
        "icons.gresource",
    );
}
