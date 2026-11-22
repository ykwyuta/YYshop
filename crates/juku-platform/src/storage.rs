//! セーブデータの保存先。Windows はファイル、Web は localStorage。

/// キーごとにバイト列を保存する。
pub struct Storage {
    #[cfg(not(target_arch = "wasm32"))]
    dir: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl Storage {
    pub fn new() -> Self {
        let base = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        Storage { dir: base.join("YYshop").join("saves") }
    }

    fn path(&self, key: &str) -> std::path::PathBuf {
        let safe: String =
            key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        self.dir.join(format!("{safe}.sav"))
    }

    pub fn load(&self, key: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(key)).ok()
    }

    pub fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        // 書き込み途中で落ちても壊れないよう、一時ファイルに書いてから置き換える
        let path = self.path(key);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    pub fn remove(&mut self, key: &str) {
        let _ = std::fs::remove_file(self.path(key));
    }
}

#[cfg(target_arch = "wasm32")]
impl Storage {
    pub fn new() -> Self {
        Storage {}
    }

    fn local() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load(&self, key: &str) -> Option<Vec<u8>> {
        let s = Self::local()?.get_item(&format!("yyshop.{key}")).ok()??;
        base64_decode(&s)
    }

    pub fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let st = Self::local().ok_or("localStorage が使えません")?;
        st.set_item(&format!("yyshop.{key}"), &base64_encode(data)).map_err(|e| format!("{e:?}"))
    }

    pub fn remove(&mut self, key: &str) {
        if let Some(st) = Self::local() {
            let _ = st.remove_item(&format!("yyshop.{key}"));
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[allow(dead_code)]
pub(crate) fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

#[allow(dead_code)]
pub(crate) fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = B64.iter().position(|&x| x == c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// `reload` が true ならブラウザの HTTP キャッシュを使わずに取り直す。
#[cfg(target_arch = "wasm32")]
pub(crate) async fn fetch_bytes(url: &str, reload: bool) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    if reload {
        init.set_cache(web_sys::RequestCache::Reload);
    }
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(url, &init))
        .await
        .map_err(|e| format!("fetch {url}: {e:?}"))?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "not a Response")?;
    if !resp.ok() {
        return Err(format!("fetch {url}: HTTP {}", resp.status()));
    }
    let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// 強制再読み込みの直後か（フォントなどをキャッシュを使わずに取り直す）。印は一度で消す。
#[cfg(target_arch = "wasm32")]
const FORCE_KEY: &str = "yyshop.force_refetch";

#[cfg(target_arch = "wasm32")]
pub(crate) fn take_force_refetch() -> bool {
    let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else { return false };
    let on = st.get_item(FORCE_KEY).ok().flatten().is_some();
    if on {
        let _ = st.remove_item(FORCE_KEY);
    }
    on
}

/// キャッシュを強制的に削除して再読み込みする（Web のみ）。セーブデータ（localStorage）は消さない。
///
/// 1. Cache Storage を全て削除（Service Worker などが作ったキャッシュ）
/// 2. Service Worker を登録解除
/// 3. 次回起動時にフォントを HTTP キャッシュを使わず取り直す印をつける
/// 4. URL にタイムスタンプを付けて再読み込み（index.html を必ず取り直す）
pub(crate) fn force_reload() {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        let Some(window) = web_sys::window() else { return };
        if let Ok(Some(st)) = window.local_storage() {
            let _ = st.set_item(FORCE_KEY, "1");
        }
        if let Ok(caches) = window.caches() {
            if let Ok(keys) = JsFuture::from(caches.keys()).await {
                for key in js_sys::Array::from(&keys).iter() {
                    if let Some(k) = key.as_string() {
                        let _ = JsFuture::from(caches.delete(&k)).await;
                    }
                }
            }
        }
        let sw = window.navigator().service_worker();
        if let Ok(regs) = JsFuture::from(sw.get_registrations()).await {
            for reg in js_sys::Array::from(&regs).iter() {
                if let Ok(reg) = reg.dyn_into::<web_sys::ServiceWorkerRegistration>() {
                    if let Ok(p) = reg.unregister() {
                        let _ = JsFuture::from(p).await;
                    }
                }
            }
        }
        let loc = window.location();
        let path = loc.pathname().unwrap_or_else(|_| "/".into());
        let _ = loc.replace(&format!("{path}?reload={}", js_sys::Date::now() as u64));
    });
}

/// 読み込み中表示を消す（Web のみ）。
pub(crate) fn hide_loading() {
    #[cfg(target_arch = "wasm32")]
    if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("loading")) {
        el.remove();
    }
}

/// 起動失敗を表示する（Web のみ）。
#[allow(unused_variables)]
pub(crate) fn show_error(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("loading")) {
        el.set_text_content(Some(&format!("起動できませんでした：{msg}")));
    }
}

/// 動作中に起きた致命的なエラーを画面に重ねて表示する（Web のみ）。
/// 読み込み中表示がもう消えていれば作り直す。
#[allow(unused_variables)]
pub fn show_fatal(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let el = match doc.get_element_by_id("fatal") {
            Some(el) => el,
            None => {
                let Ok(el) = doc.create_element("div") else { return };
                el.set_id("fatal");
                let _ = el.set_attribute(
                    "style",
                    "position:fixed;left:0;right:0;bottom:0;max-height:50%;overflow:auto;padding:12px 16px;\
                     background:#3A1414;color:#FFD9D9;font:13px/1.5 system-ui,sans-serif;white-space:pre-wrap;z-index:10",
                );
                if let Some(body) = doc.body() {
                    let _ = body.append_child(&el);
                }
                el
            }
        };
        let prev = el.text_content().unwrap_or_default();
        if prev.len() < 4000 {
            let text = if prev.is_empty() {
                format!("エラーが発生しました。この画面を開発者に送ってください。\n{msg}")
            } else {
                format!("{prev}\n{msg}")
            };
            el.set_text_content(Some(&text));
        }
    }
}

/// URL のクエリに `needle`（例 "gpu=webgpu"）が含まれるか（Web のみ）。
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn query_has(needle: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.location().search().ok()).is_some_and(|q| q.contains(needle))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = needle;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for n in 0..50 {
            let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(base64_decode(&base64_encode(&data)).unwrap(), data);
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
    }
}
