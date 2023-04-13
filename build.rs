fn main() {
    glib_build_tools::compile_resources(
        &["resources/data/"],
        "resources/data/resources.gresource.xml",
        "resources.gresource",
    );
}
