//! wgpu ベースの 2D バッチレンダラ。
//!
//! - すべての図形（角丸矩形・円弧・線・グリフ）をインスタンス化した四角形として 1 回で描く
//! - 形はフラグメントシェーダの SDF で作るので、どの解像度でもアンチエイリアスが効く
//! - 文字は fontdue でラスタライズし、動的なグリフアトラスに詰める

mod draw;
mod image;
mod renderer;
mod text;

pub use draw::{Color, DrawList, Instance, Rect};
pub use image::{decode_png, RgbaImage};
pub use renderer::Renderer;
pub use text::{FontWeight, TextSystem, ATLAS_SIZE};
pub use wgpu;
