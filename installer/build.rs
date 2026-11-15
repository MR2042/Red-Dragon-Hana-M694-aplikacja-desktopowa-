fn main() {
    // Instalator wbudowuje gotowy plik aplikacji – trzeba go najpierw zbudować w hana-rs.
    let app = "../hana-rs/target/release/hana_control.exe";
    if !std::path::Path::new(app).exists() {
        panic!("Brak {app}. Najpierw zbuduj aplikację: cd hana-rs && cargo build --release");
    }
    println!("cargo:rerun-if-changed={app}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../hana-rs/assets/icon.ico");
        res.set("FileDescription", "Instalator Hana Control");
        res.set("ProductName", "Hana Control");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Nie udało się dodać ikony: {e}");
        }
    }
}
