//! Windows 向けビルドのとき、exe にアイコンとバージョン情報を埋め込む。
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // .rc ファイルは OUT_DIR に作られるため、アイコンは絶対パスで渡す
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico = manifest.join("../../assets/icon/icon.ico").canonicalize().expect("icon.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("utf-8 path"))
        .set("ProductName", "合格の街 〜塾長シミュレーター〜")
        .set("FileDescription", "中学受験塾 経営シミュレーション")
        .set("LegalCopyright", "YYshop");
    // リソースコンパイラ（rc.exe / windres）が無い環境でもビルド自体は通す
    if let Err(e) = res.compile() {
        println!("cargo:warning=アイコンを埋め込めませんでした: {e}");
    }
}
