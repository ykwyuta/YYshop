//! 株式上場。条件を満たしたら 1 年の上場準備（監査・体制づくり）を経て上場し、資金を調達する。
//! 上場後は利益とブランドで株価が動き、塾長の持ち株が個人資産になる。

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::model::Yen;

/// 上場準備の期間（週）と費用（週あたり：監査法人・証券会社・管理体制）。
pub const IPO_PREP_WEEKS: u32 = 52;
pub const IPO_PREP_WEEKLY: Yen = 500_000;
/// 上場維持費（週あたり：監査・IR・開示）。
pub const LISTED_WEEKLY: Yen = 300_000;
/// 塾長の持ち株と、上場時に新しく発行する株数。
pub const FOUNDER_SHARES: u64 = 8_000_000;
pub const IPO_NEW_SHARES: u64 = 2_000_000;
/// 公開価格の割引（初値が上がるよう、理論株価より安く売り出す）と手数料。
pub const IPO_DISCOUNT: f64 = 0.85;
pub const IPO_FEE: f64 = 0.07;
/// 株価の記録（週）。
const PRICE_HISTORY: usize = 156;

/// 上場の条件。
pub const REQ_CLASSROOMS: usize = 4;
pub const REQ_PROFIT: Yen = 80_000_000;
pub const REQ_TRUST: f64 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListingStatus {
    #[default]
    Private,
    /// 上場準備中（あと何週）
    Preparing {
        weeks_left: u32,
    },
    Listed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Listing {
    pub status: ListingStatus,
    pub listed_on: Option<Date>,
    /// 発行済み株式数
    pub shares: u64,
    pub founder_shares: u64,
    /// 時価総額
    pub market_cap: Yen,
    /// 公開価格（円/株）
    pub ipo_price: f64,
    /// 上場で調達した資金
    pub raised: Yen,
    /// 株価の推移（円/株、週ごと）
    pub price_history: Vec<f32>,
    /// 上場審査に落ちた回数
    pub failed_attempts: u32,
}

impl Listing {
    pub fn is_listed(&self) -> bool {
        self.status == ListingStatus::Listed
    }

    pub fn is_preparing(&self) -> bool {
        matches!(self.status, ListingStatus::Preparing { .. })
    }

    /// 株価（円/株）。
    pub fn price(&self) -> f64 {
        if self.shares == 0 {
            0.0
        } else {
            self.market_cap as f64 / self.shares as f64
        }
    }

    pub fn record_price(&mut self) {
        self.price_history.push(self.price() as f32);
        if self.price_history.len() > PRICE_HISTORY {
            self.price_history.remove(0);
        }
    }
}

/// 上場条件の 1 項目。
#[derive(Clone, Debug)]
pub struct Requirement {
    pub label: String,
    pub detail: String,
    pub ok: bool,
}
