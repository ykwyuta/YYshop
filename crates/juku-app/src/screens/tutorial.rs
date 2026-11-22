//! チュートリアル：秘書が画面を順に案内する。案内している場所だけを明るく残し、ほかを暗くする。

use juku_render::{Color, Rect};
use juku_sim::World;
use juku_ui::{ButtonStyle, Ui};

use crate::screens::secretary::{self, Button};
use crate::Tab;

/// 案内する場所。
#[derive(Clone, Copy)]
pub enum Spot {
    None,
    /// ヘッダーの資金と生徒数
    Money,
    /// 「次の週へ」ボタン
    NextWeek,
    /// 教室長のひとことと秘書ボタン
    Messages,
    /// タブ（とその画面）
    Tab(Tab),
}

struct Step {
    title: &'static str,
    body: fn(&World) -> String,
    spot: Spot,
    /// この手順では 1 週進める
    advance: bool,
}

fn steps() -> Vec<Step> {
    vec![
        Step {
            title: "はじめまして、塾長",
            body: |w| {
                let c = &w.classrooms[0];
                format!(
                    "秘書の{}です。あなたの塾「{}」は、{}で生徒 {} 人・講師 {} 人の小さな進学塾です。生徒を育てて合格実績を積み上げ、地域一番の塾、そして上場企業を目指しましょう。まずは画面の見方をご案内します。",
                    secretary::NAME,
                    w.company_name,
                    w.areas[c.area].name,
                    c.students.len(),
                    c.teachers.len()
                )
            },
            spot: Spot::None,
            advance: false,
        },
        Step {
            title: "会社の資金と生徒数",
            body: |w| {
                format!(
                    "いまの資金は {} です。授業料が入り、講師の給与や家賃が出ていきます。資金がマイナスのまま 26 週続くと倒産なので、ここはいつも気にしてください。",
                    crate::fmt::format_yen(w.cash)
                )
            },
            spot: Spot::Money,
            advance: false,
        },
        Step {
            title: "1 週間ずつ進みます",
            body: |_| {
                "「次の週へ」を押すと 1 週間が過ぎ、入塾・退塾・授業・お金の出入りが計算されます。進めるたびに自動で保存されます。塾の 1 年は 2 月に始まり、翌年 2 月 1 日の入試がいちばんの勝負です。".into()
            },
            spot: Spot::NextWeek,
            advance: false,
        },
        Step {
            title: "教室長と、わたし",
            body: |w| {
                format!(
                    "左は{}教室長のひとことです。右の「秘書」ボタンを押せば、わたしが競合の動きや教室の問題を具体的にご報告します。大事なことが起きたときは、週を進めたあとに自分からお知らせしますね。",
                    w.classrooms[0].manager.name
                )
            },
            spot: Spot::Messages,
            advance: false,
        },
        Step {
            title: "教室：講師と生徒",
            body: |w| {
                let st = w.stats(0);
                format!(
                    "保護者の口コミ、気になる生徒、教室長、講師が並んでいます。いちばん大事なのは講師の「充足率」で、いまは {:.0}% です。100% を下回ると授業の質が落ち、講師が疲れて辞めてしまいます。生徒が増えたら採用しましょう。",
                    st.coverage * 100.0
                )
            },
            spot: Spot::Tab(Tab::Classroom),
            advance: false,
        },
        Step {
            title: "コース：月謝と宿題",
            body: |w| {
                format!(
                    "コースの月謝と、宿題の量を決めます。5 年の月謝はいま {} です。高すぎると生徒が離れ、安すぎると利益が出ません。宿題を増やすと学力は伸びますが、ストレスで退塾が増えます。",
                    crate::fmt::format_yen(w.classrooms[0].main_fee(5))
                )
            },
            spot: Spot::Tab(Tab::Courses),
            advance: false,
        },
        Step {
            title: "集客：競合と施策",
            body: |w| {
                let rivals = w.area_shares(0).iter().filter(|s| !s.3).count();
                format!(
                    "けやき台には競合が {rivals} 社います。地域のシェアと、競合の動き（値下げ・広告・出店）が見られます。無料体験やチラシなどの施策で、生徒を呼び込みましょう。"
                )
            },
            spot: Spot::Tab(Tab::Marketing),
            advance: false,
        },
        Step {
            title: "経理：お金の流れ",
            body: |_| {
                "毎週の損益計算書です。授業料などの売上から、講師の人件費・家賃・広告費を引いたものが利益になります。前週と比べて、どこが増えて、どこが減ったかを確かめましょう。".into()
            },
            spot: Spot::Tab(Tab::Accounting),
            advance: false,
        },
        Step {
            title: "本部：会社を大きくする",
            body: |_| {
                "新しい教室の開校や塾の買収、銀行融資、物件、自社教材、スター講師の引き抜き、そして株式上場はここから。教室が 3 つになると北部、上場すると都心にも出店できます。".into()
            },
            spot: Spot::Tab(Tab::Hq),
            advance: false,
        },
        Step {
            title: "最初の 1 週間を進めてみましょう",
            body: |_| "準備はできました。下のボタンで、最初の 1 週間を進めてみましょう。".into(),
            spot: Spot::NextWeek,
            advance: true,
        },
        Step {
            title: "1 週間が過ぎました",
            body: |w| {
                let r = w.classrooms[0].last_report();
                let (new, net) = r.map(|r| (r.new_students, r.pnl.net_profit())).unwrap_or((0, 0));
                format!(
                    "この 1 週間で {new} 人が入塾し、教室の純利益は {} でした。詳しくは経理タブで見られます。気になることがあれば、わたしからお知らせします。それでは塾長、よろしくお願いします！",
                    crate::fmt::format_yen(net)
                )
            },
            spot: Spot::None,
            advance: false,
        },
    ]
}

/// チュートリアルの進み具合。
pub struct Tutorial {
    pub step: usize,
}

pub enum Outcome {
    /// まだ続く
    Continue,
    /// 終わった（またはスキップした）
    Done,
}

/// 画面の配置（GameApp が持つ値）。
pub struct Layout {
    pub screen: Rect,
    pub x0: f32,
    pub w: f32,
    pub header_h: f32,
    pub tab_h: f32,
}

impl Layout {
    fn spot_rect(&self, spot: Spot) -> Option<Rect> {
        let s = self.screen;
        let pad = 16.0;
        match spot {
            Spot::None => None,
            Spot::Money => Some(Rect::new(self.x0 + pad - 6.0, s.y + 54.0, 260.0, 48.0)),
            Spot::NextWeek => Some(Rect::new(self.x0 + self.w - pad - 134.0, s.y + 54.0, 140.0, 48.0)),
            Spot::Messages => Some(Rect::new(self.x0 + pad - 6.0, s.y + 100.0, self.w - pad * 2.0 + 12.0, 48.0)),
            Spot::Tab(tab) => {
                let n = Tab::ALL.len() as f32;
                let cw = self.w / n;
                let i = Tab::ALL.iter().position(|t| *t == tab).unwrap_or(0) as f32;
                Some(Rect::new(self.x0 + cw * i, s.bottom() - self.tab_h, cw, self.tab_h))
            }
        }
    }
}

impl Tutorial {
    pub fn new() -> Self {
        Tutorial { step: 0 }
    }

    /// 今の手順で見せたいタブ。
    pub fn tab(&self) -> Option<Tab> {
        match steps().get(self.step).map(|s| s.spot) {
            Some(Spot::Tab(t)) => Some(t),
            _ => None,
        }
    }

    pub fn draw(&mut self, ui: &mut Ui, world: &mut World, layout: &Layout) -> Outcome {
        let all = steps();
        let Some(step) = all.get(self.step) else { return Outcome::Done };
        let t = ui.theme;
        let s = layout.screen;

        // 暗幕（案内する場所だけ明るく残す）
        let shade = Color::BLACK.with_alpha(0.62);
        match layout.spot_rect(step.spot) {
            // タブの案内では、タブとその画面（ヘッダーとタブバーの間）を明るく残す
            Some(h) if matches!(step.spot, Spot::Tab(_)) => {
                let bar_y = s.bottom() - layout.tab_h;
                ui.dl.rect(Rect::new(s.x, s.y, s.w, layout.header_h), shade, 0.0);
                ui.dl.rect(Rect::new(s.x, bar_y, (h.x - s.x).max(0.0), layout.tab_h), shade, 0.0);
                ui.dl.rect(Rect::new(h.right(), bar_y, (s.right() - h.right()).max(0.0), layout.tab_h), shade, 0.0);
                let pulse = 0.55 + 0.45 * ((ui.input.time * 3.0).sin() as f32 * 0.5 + 0.5);
                ui.dl.rect_border(h, Color::TRANSPARENT, 12.0, 3.0, t.star.with_alpha(pulse));
                ui.mem.request_repaint();
            }
            Some(h) => {
                ui.dl.rect(Rect::new(s.x, s.y, s.w, (h.y - s.y).max(0.0)), shade, 0.0);
                ui.dl.rect(Rect::new(s.x, h.bottom(), s.w, (s.bottom() - h.bottom()).max(0.0)), shade, 0.0);
                ui.dl.rect(Rect::new(s.x, h.y, (h.x - s.x).max(0.0), h.h), shade, 0.0);
                ui.dl.rect(Rect::new(h.right(), h.y, (s.right() - h.right()).max(0.0), h.h), shade, 0.0);
                // 枠を点滅させる
                let pulse = 0.55 + 0.45 * ((ui.input.time * 3.0).sin() as f32 * 0.5 + 0.5);
                ui.dl.rect_border(h, Color::TRANSPARENT, 12.0, 3.0, t.star.with_alpha(pulse));
                ui.mem.request_repaint();
            }
            None => ui.dl.rect(s, shade, 0.0),
        }

        // 秘書のウインドウ（タブバーのすぐ上）
        let bottom = s.bottom() - layout.tab_h;
        let area = Rect::new(layout.x0, s.y + layout.header_h, layout.w, bottom - s.y - layout.header_h);
        let body = (step.body)(world);
        let last = self.step + 1 >= all.len();
        let next_label = if step.advance {
            "次の週へ進める"
        } else if last {
            "はじめる"
        } else {
            "次へ"
        };
        let mut buttons = Vec::new();
        if !last {
            buttons.push(Button { label: "スキップ", style: ButtonStyle::Ghost });
        }
        buttons.push(Button { label: next_label, style: ButtonStyle::Primary });
        let clicked = secretary::window(ui, area, None, step.title, &body, Some((self.step, all.len())), &buttons);
        match clicked {
            Some(i) if !last && i == 0 => Outcome::Done,
            Some(_) => {
                if step.advance {
                    if world.pending_event.is_some() {
                        world.resolve_event(juku_sim::Choice::B);
                    }
                    world.step_week();
                }
                self.step += 1;
                if self.step >= all.len() {
                    Outcome::Done
                } else {
                    Outcome::Continue
                }
            }
            None => Outcome::Continue,
        }
    }
}

impl Default for Tutorial {
    fn default() -> Self {
        Self::new()
    }
}
