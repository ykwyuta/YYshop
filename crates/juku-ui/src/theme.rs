//! 配色と文字サイズ。黒板をモチーフにした深緑×チョーク×琥珀色。

use juku_render::Color;

#[derive(Clone, Debug)]
pub struct Theme {
    /// 画面の背景
    pub bg: Color,
    /// ヘッダー・タブバー
    pub bar: Color,
    /// セクションの帯
    pub band: Color,
    /// カード
    pub card: Color,
    pub card_border: Color,
    pub text: Color,
    pub text_dim: Color,
    /// アクセント（ボタン・トグル）
    pub accent: Color,
    pub accent_dark: Color,
    pub positive: Color,
    pub negative: Color,
    pub star: Color,
    pub track: Color,
    /// グラフ系列の色
    pub series: [Color; 6],

    pub font_small: f32,
    pub font_body: f32,
    pub font_heading: f32,
    pub font_title: f32,
    pub font_number: f32,
    pub radius: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            bg: Color::hex(0x1E2B26),
            bar: Color::hex(0x14201B),
            band: Color::hex(0x0F1814),
            card: Color::hex(0x15211C),
            card_border: Color::hex(0x3A4F45),
            text: Color::hex(0xF3F1E7),
            text_dim: Color::hex(0x9DB0A6),
            accent: Color::hex(0xE39B3A),
            accent_dark: Color::hex(0x8A5A1C),
            positive: Color::hex(0x6CC46A),
            negative: Color::hex(0xF06A5A),
            star: Color::hex(0xF5B82E),
            track: Color::hex(0x0B120F),
            series: [
                Color::hex(0xE39B3A),
                Color::hex(0x4F8CF0),
                Color::hex(0xF2B63A),
                Color::hex(0xA86BE8),
                Color::hex(0x41C6C0),
                Color::hex(0xF06A8C),
            ],
            font_small: 12.0,
            font_body: 14.0,
            font_heading: 17.0,
            font_title: 20.0,
            font_number: 30.0,
            radius: 14.0,
        }
    }
}
