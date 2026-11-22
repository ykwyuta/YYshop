//! 画像の読み込み状態。画面コードは毎フレーム `ui.image(path)` で状態を尋ね、
//! まだなら読み込みを頼む。読み込み（ファイル・fetch・GPU への転送）はプラットフォーム層が行う。

use std::collections::HashMap;

/// GPU に載った画像。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub id: u32,
    pub width: u32,
    pub height: u32,
}

/// 画像の状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageState {
    /// 読み込み中
    Loading,
    Ready(ImageInfo),
    /// ファイルが無い・読めない
    Missing,
}

enum Slot {
    Requested,
    Loaded { info: ImageInfo, last_used: u64 },
    Missing,
}

/// 画像のパス → 状態。
#[derive(Default)]
pub struct Images {
    slots: HashMap<String, Slot>,
    requests: Vec<String>,
    frame: u64,
    next_id: u32,
}

impl Images {
    /// 画像の状態を返す。初めて尋ねられた画像は読み込みを頼む。
    pub fn get(&mut self, path: &str) -> ImageState {
        let frame = self.frame;
        match self.slots.get_mut(path) {
            Some(Slot::Loaded { info, last_used }) => {
                *last_used = frame;
                ImageState::Ready(*info)
            }
            Some(Slot::Requested) => ImageState::Loading,
            Some(Slot::Missing) => ImageState::Missing,
            None => {
                self.slots.insert(path.to_string(), Slot::Requested);
                self.requests.push(path.to_string());
                ImageState::Loading
            }
        }
    }

    /// フレームの始まり（最後に使ったフレームを数えるため）。
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// 読み込みを頼まれた画像のパス（プラットフォーム層が取り出す）。
    pub fn take_requests(&mut self) -> Vec<String> {
        std::mem::take(&mut self.requests)
    }

    /// 読み込めた画像に id を振る。この id で GPU に載せる。
    pub fn loaded(&mut self, path: &str, width: u32, height: u32) -> ImageInfo {
        self.next_id += 1;
        let info = ImageInfo { id: self.next_id, width, height };
        self.slots.insert(path.to_string(), Slot::Loaded { info, last_used: self.frame });
        info
    }

    pub fn failed(&mut self, path: &str) {
        self.slots.insert(path.to_string(), Slot::Missing);
    }

    /// GPU に載せておく画像を `keep` 枚までにする。外した画像の id を返す（このフレームで使った画像は残す）。
    pub fn evict(&mut self, keep: usize) -> Vec<u32> {
        let mut loaded: Vec<(u64, String, u32)> = self
            .slots
            .iter()
            .filter_map(|(p, s)| match s {
                Slot::Loaded { info, last_used } => Some((*last_used, p.clone(), info.id)),
                _ => None,
            })
            .collect();
        if loaded.len() <= keep {
            return Vec::new();
        }
        loaded.sort();
        let excess = loaded.len() - keep;
        let mut out = Vec::new();
        for (last_used, path, id) in loaded.into_iter().take(excess) {
            if last_used >= self.frame {
                break;
            }
            // 外した画像は、また使われたら読み込み直す
            self.slots.remove(&path);
            out.push(id);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_load_and_evict() {
        let mut im = Images::default();
        assert_eq!(im.get("a.png"), ImageState::Loading);
        assert_eq!(im.get("a.png"), ImageState::Loading);
        assert_eq!(im.take_requests(), vec!["a.png".to_string()]);
        let a = im.loaded("a.png", 10, 5);
        im.begin_frame();
        im.get("b.png");
        im.loaded("b.png", 4, 4);
        assert_eq!(im.get("b.png"), ImageState::Ready(ImageInfo { id: a.id + 1, width: 4, height: 4 }));
        assert_eq!(im.evict(1), vec![a.id], "古い画像から外す");
        assert_eq!(im.get("a.png"), ImageState::Loading, "外した画像はまた頼む");
        im.failed("c.png");
        assert_eq!(im.get("c.png"), ImageState::Missing);
    }
}
