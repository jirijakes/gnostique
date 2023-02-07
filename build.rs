fn main() {
    glib_build_tools::compile_resources(
        "resources",
        "resources/gnostique.gresource.xml",
        "gnostique.gresource",
    );
}
