// Компиляция GResource (style.css, иконки) в бинарный .gresource,
// который встраивается в приложение и регистрируется в main.rs.
fn main() {
    glib_build_tools::compile_resources(
        &["data"],
        "data/resources.gresource.xml",
        "host42.gresource",
    );
}
