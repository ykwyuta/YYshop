//! 人物イラストの選択と描画。表情が変わっても人物の絵柄は変えない。

use juku_render::Rect;
use juku_sim::{Manager, Student, Teacher, World};
use juku_ui::{Id, Ui};

pub const ATLAS_PNGS: &[&[u8]] = &[
    include_bytes!("../../../assets/portraits/portraits.png"),
    include_bytes!("../../../assets/portraits/teachers.png"),
];

const VARIANTS: usize = 20;
const EXTRA_TEACHERS: usize = 30;
const EXPRESSIONS: usize = 5;
const TILE_PX: f32 = 96.0;
const SECRETARY_PX: f32 = 192.0;
const SECRETARY_Y: f32 = TILE_PX * (3 * EXPRESSIONS) as f32;
const EXTRA_TEACHER_Y: f32 = SECRETARY_Y + SECRETARY_PX;
const EXTRA_TEACHER_PX: f32 = 64.0;
const ATLAS_W: f32 = TILE_PX * VARIANTS as f32;
const ATLAS_H: f32 = EXTRA_TEACHER_Y + EXTRA_TEACHER_PX * EXPRESSIONS as f32;

#[derive(Clone, Copy)]
enum Role {
    Student,
    Teacher,
    Manager,
    Secretary,
}

#[derive(Clone, Copy)]
pub enum Expression {
    Neutral,
    Happy,
    Worried,
    Surprised,
    Focused,
}

#[derive(Clone, Copy)]
pub struct Portrait {
    role: Role,
    variant: usize,
    expression: Expression,
}

impl Portrait {
    fn new(role: Role, id: Id, expression: Expression) -> Self {
        let mixed = id.0 ^ (id.0 >> 32);
        let variants = match role {
            Role::Teacher => VARIANTS + EXTRA_TEACHERS,
            _ => VARIANTS,
        };
        Self { role, variant: (mixed % variants as u64) as usize, expression }
    }

    pub fn student(s: &Student) -> Self {
        let expression = if s.stress >= 0.65 || s.motivation < 0.35 {
            Expression::Worried
        } else if s.history.iter().rev().nth(1).is_some_and(|previous| s.deviation - f64::from(*previous) > 2.0) {
            Expression::Surprised
        } else if s.motivation >= 0.75 {
            Expression::Happy
        } else if s.motivation >= 0.55 {
            Expression::Focused
        } else {
            Expression::Neutral
        };
        Self::new(Role::Student, Id::new(&s.name).with(s.id.into()), expression)
    }

    pub fn teacher(t: &Teacher) -> Self {
        let expression = if t.fatigue >= 0.7 {
            Expression::Worried
        } else if t.fatigue >= 0.4 {
            Expression::Focused
        } else if t.star && t.loyal_weeks >= 49 {
            Expression::Surprised
        } else if t.star {
            Expression::Happy
        } else {
            Expression::Neutral
        };
        Self::new(Role::Teacher, Id::new(&t.name).with(t.id.into()), expression)
    }

    pub fn manager(m: &Manager, expression: Expression) -> Self {
        // 採用の前後で、就任日や年俸が変わっても同じ絵になるようにする。
        Self::new(Role::Manager, Id::new(&m.name).with(m.staffing.into()), expression)
    }

    pub fn secretary(expression: Expression) -> Self {
        Self { role: Role::Secretary, variant: 0, expression }
    }

    pub fn with_expression(self, expression: Expression) -> Self {
        Self { expression, ..self }
    }

    pub fn draw(self, ui: &mut Ui, bounds: Rect) {
        let size = bounds.w.min(bounds.h);
        let (cx, cy) = bounds.center();
        let r = Rect::new(cx - size * 0.5, cy - size * 0.5, size, size);
        let expression = self.expression as usize;
        let (x, y, tile) = match self.role {
            Role::Secretary => (expression as f32 * SECRETARY_PX, SECRETARY_Y, SECRETARY_PX),
            Role::Teacher if self.variant >= VARIANTS => (
                (self.variant - VARIANTS) as f32 * EXTRA_TEACHER_PX,
                EXTRA_TEACHER_Y + expression as f32 * EXTRA_TEACHER_PX,
                EXTRA_TEACHER_PX,
            ),
            _ => {
                let x = self.variant as f32 * TILE_PX;
                let y = (self.role as usize * EXPRESSIONS + expression) as f32 * TILE_PX;
                (x, y, TILE_PX)
            }
        };
        // 線形補間で隣のコマの色が混ざらないよう、端の画素の中心までを使う。
        let uv = [(x + 0.5) / ATLAS_W, (y + 0.5) / ATLAS_H, (x + tile - 0.5) / ATLAS_W, (y + tile - 0.5) / ATLAS_H];
        ui.dl.sprite(r, uv, (size * 0.16).min(12.0));
    }
}

pub fn manager_expression(world: &World, ci: usize) -> Expression {
    let c = &world.classrooms[ci];
    if world.cash < 0 || c.last_report().is_some_and(|r| r.coverage < 0.95 && !c.students.is_empty()) {
        Expression::Worried
    } else if world.pending_event.is_some() {
        Expression::Surprised
    } else if c.students.is_empty() {
        Expression::Focused
    } else if c.last_report().is_none() {
        Expression::Neutral
    } else {
        Expression::Happy
    }
}
