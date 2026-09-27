slint::slint! {
    export component AppWindow inherits Window {
        title: "MSSQL Schema Compare";
        preferred-width: 1100px; preferred-height: 720px;
        Text { text: "SchemaDiff"; }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    AppWindow::new()?.run()
}
