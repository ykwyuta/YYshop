//! ポインタ（マウス・タッチ）入力の状態。プラットフォーム層がイベントを流し込む。

/// これ以上動いたらクリックではなくドラッグとみなす（論理 px）。
const DRAG_THRESHOLD: f32 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// 現在のポインタ位置（論理 px）。タッチが離れたら None。
    pub pointer: Option<(f32, f32)>,
    prev_pointer: Option<(f32, f32)>,
    /// このフレームでのポインタの移動量
    pub delta: (f32, f32),
    /// 押下中
    pub down: bool,
    /// このフレームで押された
    pub pressed: bool,
    /// このフレームで離された
    pub released: bool,
    /// 押し始めた位置
    pub press_origin: Option<(f32, f32)>,
    /// 押してから最も離れた距離
    drag_distance: f32,
    /// ホイール量（論理 px）
    pub wheel: (f32, f32),
    /// 経過時間（秒）
    pub time: f64,
    /// 前フレームからの経過（秒）
    pub dt: f32,
    /// タッチ操作（離したらポインタを消す）
    touch: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_drag(&self) -> bool {
        self.drag_distance > DRAG_THRESHOLD
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32) {
        self.pointer = Some((x, y));
        if self.down {
            if let Some((ox, oy)) = self.press_origin {
                let d = ((x - ox).powi(2) + (y - oy).powi(2)).sqrt();
                self.drag_distance = self.drag_distance.max(d);
            }
        }
    }

    pub fn pointer_down(&mut self, touch: bool) {
        self.touch = touch;
        self.down = true;
        self.pressed = true;
        self.press_origin = self.pointer;
        self.prev_pointer = self.pointer;
        self.drag_distance = 0.0;
    }

    pub fn pointer_up(&mut self) {
        if self.down {
            self.down = false;
            self.released = true;
        }
    }

    pub fn pointer_left(&mut self) {
        if !self.down {
            self.pointer = None;
        }
    }

    pub fn wheel(&mut self, dx: f32, dy: f32) {
        self.wheel.0 += dx;
        self.wheel.1 += dy;
    }

    /// フレームの始めに呼ぶ。
    pub fn begin_frame(&mut self, time: f64) {
        self.dt = if self.time > 0.0 { (time - self.time) as f32 } else { 1.0 / 60.0 };
        self.time = time;
        self.delta = match (self.pointer, self.prev_pointer) {
            (Some(p), Some(q)) if self.down => (p.0 - q.0, p.1 - q.1),
            _ => (0.0, 0.0),
        };
        self.prev_pointer = self.pointer;
    }

    /// フレームの終わりに呼ぶ。
    pub fn end_frame(&mut self) {
        self.pressed = false;
        if self.released {
            self.released = false;
            self.press_origin = None;
            self.drag_distance = 0.0;
            if self.touch {
                self.pointer = None;
                self.prev_pointer = None;
            }
        }
        self.wheel = (0.0, 0.0);
    }

    /// 入力イベントが溜まっていて描き直しが必要か。
    pub fn has_activity(&self) -> bool {
        self.pressed || self.released || self.down || self.wheel != (0.0, 0.0)
    }
}
