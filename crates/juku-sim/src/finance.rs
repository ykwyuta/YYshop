//! 銀行融資と、損益に載らないお金の動き（物件の売買・買収・借入と返済）。

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::model::Yen;

/// 銀行からの借入（元金均等返済）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loan {
    pub id: u32,
    pub principal: Yen,
    /// 残高
    pub balance: Yen,
    /// 年利
    pub rate: f64,
    pub weeks_total: u32,
    pub weeks_left: u32,
    pub taken: Date,
}

impl Loan {
    /// 毎週の元金返済額。
    pub fn weekly_principal(&self) -> Yen {
        (self.principal / self.weeks_total.max(1) as Yen).max(1)
    }

    /// 今週の利息。
    pub fn weekly_interest(&self) -> Yen {
        (self.balance as f64 * self.rate / 52.0) as Yen
    }

    /// 今週の返済額（元金＋利息）。
    pub fn weekly_payment(&self) -> Yen {
        self.weekly_principal().min(self.balance) + self.weekly_interest()
    }

    /// 1 週分を返済する。(元金, 利息) を返す。
    pub fn pay_week(&mut self) -> (Yen, Yen) {
        let interest = self.weekly_interest();
        self.weeks_left = self.weeks_left.saturating_sub(1);
        let principal = if self.weeks_left == 0 { self.balance } else { self.weekly_principal().min(self.balance) };
        self.balance -= principal;
        (principal, interest)
    }

    pub fn is_repaid(&self) -> bool {
        self.balance <= 0
    }
}

/// 損益には載らないお金の動き（その週の合計）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapitalFlows {
    pub loan_in: Yen,
    pub loan_principal: Yen,
    pub property_purchase: Yen,
    pub property_sale: Yen,
    pub acquisition: Yen,
}

impl CapitalFlows {
    pub fn lines(&self) -> [(&'static str, Yen); 5] {
        [
            ("借入", self.loan_in),
            ("借入金の返済（元金）", -self.loan_principal),
            ("物件の購入", -self.property_purchase),
            ("物件の売却", self.property_sale),
            ("塾の買収", -self.acquisition),
        ]
    }

    pub fn total(&self) -> Yen {
        self.lines().iter().map(|(_, v)| v).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.lines().iter().all(|(_, v)| *v == 0)
    }
}

/// スター講師への提示条件。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferLevel {
    /// 年俸 1.15 倍・契約金 年俸の 3 割
    Standard,
    /// 年俸 1.4 倍・契約金 年俸の 6 割
    Good,
    /// 年俸 1.8 倍・契約金 年俸と同額
    Generous,
}

impl OfferLevel {
    pub const ALL: [OfferLevel; 3] = [OfferLevel::Standard, OfferLevel::Good, OfferLevel::Generous];

    pub fn label(self) -> &'static str {
        match self {
            OfferLevel::Standard => "標準",
            OfferLevel::Good => "好条件",
            OfferLevel::Generous => "破格",
        }
    }

    pub fn pay_multiplier(self) -> f64 {
        match self {
            OfferLevel::Standard => 1.15,
            OfferLevel::Good => 1.4,
            OfferLevel::Generous => 1.8,
        }
    }

    pub fn bonus_ratio(self) -> f64 {
        match self {
            OfferLevel::Standard => 0.3,
            OfferLevel::Good => 0.6,
            OfferLevel::Generous => 1.0,
        }
    }

    pub fn chance_multiplier(self) -> f64 {
        match self {
            OfferLevel::Standard => 1.0,
            OfferLevel::Good => 1.6,
            OfferLevel::Generous => 2.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loan(principal: Yen, weeks: u32) -> Loan {
        Loan {
            id: 1,
            principal,
            balance: principal,
            rate: 0.03,
            weeks_total: weeks,
            weeks_left: weeks,
            taken: Date { year: 2027, month: 2, day: 1 },
        }
    }

    #[test]
    fn loan_is_fully_repaid_at_term() {
        let mut l = loan(10_000_000, 260);
        let mut principal = 0;
        let mut interest = 0;
        for _ in 0..260 {
            let (p, i) = l.pay_week();
            principal += p;
            interest += i;
        }
        assert!(l.is_repaid());
        assert_eq!(principal, 10_000_000);
        // 元金均等：利息は平均残高（約半分）× 年利 × 年数
        assert!(interest > 700_000 && interest < 800_000, "interest = {interest}");
    }

    #[test]
    fn interest_shrinks_as_balance_falls() {
        let mut l = loan(5_200_000, 52);
        let first = l.weekly_interest();
        for _ in 0..40 {
            l.pay_week();
        }
        assert!(l.weekly_interest() < first);
    }
}
