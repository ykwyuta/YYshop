//! 会社としての投資・財務：銀行融資、物件の売買、塾の買収、スター講師の引き抜き。

use crate::classroom::{clamp01, Classroom};
use crate::finance::{CapitalFlows, Loan, OfferLevel};
use crate::gen;
use crate::listing::{self, ListingStatus, Requirement};
use crate::market::CompetitorStyle;
use crate::materials;
use crate::model::*;
use crate::report::HqCosts;
use crate::world::World;

/// 融資の上限と下限。
pub const LOAN_MIN: Yen = 1_000_000;
/// 物件の売却手数料率。
const PROPERTY_SALE_FEE: f64 = 0.05;
/// 買収で生徒が残る割合。
const ACQUISITION_RETENTION: f64 = 0.85;

impl World {
    // ------------------------------------------------------------------
    // 銀行融資
    // ------------------------------------------------------------------

    /// 借入残高の合計。
    pub fn debt(&self) -> Yen {
        self.loans.iter().map(|l| l.balance).sum()
    }

    /// 所有物件の評価額の合計。
    pub fn property_assets(&self) -> Yen {
        self.classrooms.iter().filter(|c| c.owned).map(|c| c.property_value).sum()
    }

    /// 直近 52 週の会社の純利益。
    pub fn annual_profit(&self) -> Yen {
        self.history.iter().rev().take(52).map(|w| w.net()).sum()
    }

    /// あといくら借りられるか。利益・物件（担保）・ブランドで決まる。
    pub fn credit_limit(&self) -> Yen {
        let mut base = 10_000_000.0
            + self.annual_profit().max(0) as f64 * 3.0
            + self.property_assets() as f64 * 0.7
            + self.brand * 20_000_000.0;
        // 上場企業は信用が高い
        if self.listing.is_listed() {
            base += self.listing.market_cap as f64 * 0.1;
        }
        let limit = base * (0.5 + self.trust);
        ((limit as Yen - self.debt()).max(0) / 1_000_000) * 1_000_000
    }

    /// 新しく借りる場合の年利。
    pub fn loan_rate(&self) -> f64 {
        let mut rate = 0.022 + (1.0 - self.trust) * 0.02;
        if self.annual_profit() < 0 {
            rate += 0.02;
        }
        let assets = (self.cash + self.property_assets()).max(1) as f64;
        rate += (self.debt() as f64 / assets).min(2.0) * 0.01;
        if self.negative_weeks > 0 {
            rate += 0.02;
        }
        (rate * 1000.0).round() / 1000.0
    }

    /// 借りる。`years` 年の元金均等返済。
    pub fn borrow(&mut self, amount: Yen, years: u32) -> Result<(), String> {
        if amount < LOAN_MIN {
            return Err("借入額が少なすぎます".into());
        }
        if amount > self.credit_limit() {
            return Err("融資枠を超えています".into());
        }
        let weeks = years.max(1) * 52;
        let rate = self.loan_rate();
        let id = gen::alloc_id(&mut self.next_id);
        self.loans.push(Loan {
            id,
            principal: amount,
            balance: amount,
            rate,
            weeks_total: weeks,
            weeks_left: weeks,
            taken: self.calendar.date(),
        });
        self.cash += amount;
        self.pending_flows.loan_in += amount;
        Ok(())
    }

    /// 繰り上げ返済（手数料なし）。
    pub fn repay_loan(&mut self, id: u32) -> Result<(), String> {
        let Some(i) = self.loans.iter().position(|l| l.id == id) else {
            return Err("借入が見つかりません".into());
        };
        let balance = self.loans[i].balance;
        if self.cash < balance {
            return Err("資金が足りません".into());
        }
        self.cash -= balance;
        self.pending_flows.loan_principal += balance;
        self.loans.remove(i);
        Ok(())
    }

    /// 毎週の返済。
    pub(crate) fn service_loans(&mut self, hq: &mut HqCosts, flows: &mut CapitalFlows, news: &mut Vec<String>) {
        for l in self.loans.iter_mut() {
            let (principal, interest) = l.pay_week();
            self.cash -= principal + interest;
            hq.interest += interest;
            flows.loan_principal += principal;
            if l.is_repaid() {
                news.push(format!("{}の借入（{}）を完済しました", l.taken.label(), crate::format_yen(l.principal)));
            }
        }
        self.loans.retain(|l| !l.is_repaid());
    }

    // ------------------------------------------------------------------
    // 物件
    // ------------------------------------------------------------------

    pub fn property_price(&self, ci: usize) -> Yen {
        self.areas[self.classrooms[ci].area].property_price
    }

    /// 教室の物件を買う。以後は家賃の代わりに維持費（年 1.5%）がかかる。
    pub fn buy_property(&mut self, ci: usize) -> Result<(), String> {
        let price = self.property_price(ci);
        let c = &self.classrooms[ci];
        if c.owned {
            return Err("すでに所有しています".into());
        }
        if self.cash < price {
            return Err("資金が足りません。融資を検討してください".into());
        }
        self.cash -= price;
        self.pending_flows.property_purchase += price;
        let c = &mut self.classrooms[ci];
        c.owned = true;
        c.property_value = price;
        Ok(())
    }

    /// 物件を売る（手数料 5%）。以後はまた家賃がかかる。
    pub fn sell_property(&mut self, ci: usize) -> Result<Yen, String> {
        let c = &mut self.classrooms[ci];
        if !c.owned {
            return Err("所有していません".into());
        }
        let proceeds = (c.property_value as f64 * (1.0 - PROPERTY_SALE_FEE)) as Yen;
        c.owned = false;
        c.property_value = 0;
        self.cash += proceeds;
        self.pending_flows.property_sale += proceeds;
        Ok(proceeds)
    }

    // ------------------------------------------------------------------
    // 買収
    // ------------------------------------------------------------------

    /// 売りに出ている個人塾・中小塾（`competitors` の添字）。
    pub fn acquisition_targets(&self) -> Vec<usize> {
        (0..self.competitors.len())
            .filter(|&i| {
                let c = &self.competitors[i];
                c.for_sale && !c.acquired && !c.branches.is_empty() && !c.style.is_major()
            })
            .collect()
    }

    /// 買収価格：生徒 1 人 30 万円 ＋ 授業の質 ＋ 教室ごとの設備。
    pub fn acquisition_price(&self, comp: usize) -> Yen {
        let c = &self.competitors[comp];
        let students = c.total_students() as f64;
        let price = students * 300_000.0 + c.quality * 4_000_000.0 + c.branches.len() as f64 * 2_000_000.0;
        (price as Yen / 100_000) * 100_000
    }

    /// 塾を買収する。自塾のないエリアは新しい教室に、あるエリアは既存の教室に統合する。
    /// 結果の説明を返す。
    pub fn acquire(&mut self, comp: usize) -> Result<Vec<String>, String> {
        if !self.acquisition_targets().contains(&comp) {
            return Err("この塾は売りに出ていません".into());
        }
        let price = self.acquisition_price(comp);
        if self.cash < price {
            return Err("資金が足りません。融資を検討してください".into());
        }
        self.cash -= price;
        self.pending_flows.acquisition += price;

        let target = self.competitors[comp].clone();
        let mut summary = Vec::new();
        let date = self.calendar.date();
        for b in &target.branches {
            // 経営が変わると一部の生徒は辞める
            let mut students = Vec::new();
            for gi in 0..3 {
                let n = self.rng.round_stochastic(b.students[gi] as f64 * ACQUISITION_RETENTION);
                for _ in 0..n {
                    let mut s = gen::make_student(&mut self.rng, &mut self.next_id, &self.schools, gi as u8 + 4, false);
                    s.weeks_enrolled = 10 + self.rng.below(80) as u32;
                    students.push(s);
                }
            }
            let total = students.len() as u32;
            let n_teachers = total.div_ceil(12).max(1);
            let mut teachers = Vec::new();
            for k in 0..n_teachers {
                let pro = k == 0;
                teachers.push(gen::make_inherited_teacher(&mut self.rng, &mut self.next_id, target.quality, pro));
            }
            let area_name = self.areas[b.area].name.clone();

            if let Some(ci) = self.classrooms.iter().position(|c| c.area == b.area) {
                let room = &mut self.classrooms[ci];
                room.students.extend(students);
                room.teachers.extend(teachers);
                room.awareness = clamp01(room.awareness + 0.05);
                summary
                    .push(format!("{}の生徒 {total} 人と講師 {n_teachers} 人を{}に迎えました", target.name, room.name));
            } else {
                let used: Vec<String> = self.classrooms.iter().map(|c| c.manager.name.clone()).collect();
                let mut manager = gen::make_manager(&mut self.rng, date, &used);
                if target.style == CompetitorStyle::Independent {
                    // 個人塾の元塾長がそのまま教室長になる：指導と面倒見は確か、営業は苦手
                    manager.teaching = manager.teaching.max(80);
                    manager.ethics = manager.ethics.max(75);
                    manager.sales = manager.sales.min(55);
                }
                let id = gen::alloc_id(&mut self.next_id);
                let a = &self.areas[b.area];
                let mut room = Classroom::new(id, format!("{}校", a.name), b.area, a.address.clone(), date, manager);
                room.students = students;
                room.teachers = teachers;
                room.candidates = gen::make_candidates(&mut self.rng, &mut self.next_id);
                room.awareness = clamp01(b.awareness * 0.8);
                room.review_count = 20 + total;
                room.ratings = Ratings { price: 3.4, teaching: 3.3, care: 3.6, safety: 3.3 };
                for (course, fee) in room.courses.iter_mut().zip(b.fees.iter()) {
                    course.fee = *fee;
                }
                summary.push(format!(
                    "{}を{}校として引き継ぎました（生徒 {total} 人・講師 {n_teachers} 人）",
                    target.name, area_name
                ));
                self.classrooms.push(room);
            }
        }

        let c = &mut self.competitors[comp];
        c.acquired = true;
        c.for_sale = false;
        c.branches.clear();
        c.stars.clear();
        Ok(summary)
    }

    // ------------------------------------------------------------------
    // スター講師の引き抜き
    // ------------------------------------------------------------------

    /// 声をかけられるスター講師：(競合の添字, 講師の添字)。
    pub fn star_targets(&self) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for (ci, c) in self.competitors.iter().enumerate() {
            if c.acquired {
                continue;
            }
            for si in 0..c.stars.len() {
                v.push((ci, si));
            }
        }
        v
    }

    /// 引き抜きの成功確率。
    pub fn star_offer_chance(&self, comp: usize, level: OfferLevel) -> f64 {
        let c = &self.competitors[comp];
        let base = c.style.star_base_chance() * level.chance_multiplier();
        (base + (self.brand - c.brand) * 0.25).clamp(0.05, 0.92)
    }

    /// 年俸と契約金。
    pub fn star_offer_terms(&self, comp: usize, star: usize, level: OfferLevel) -> (Yen, Yen) {
        let pay = self.competitors[comp].stars[star].pay as f64;
        let salary = ((pay * level.pay_multiplier()) as Yen / 100_000) * 100_000;
        let bonus = ((pay * level.bonus_ratio()) as Yen / 100_000) * 100_000;
        (salary, bonus)
    }

    /// スター講師に声をかける。成功したら Ok(true)、断られたら Ok(false)。
    /// 契約金は成功したときだけ払う。
    pub fn offer_star(&mut self, comp: usize, star: usize, ci: usize, level: OfferLevel) -> Result<bool, String> {
        if ci >= self.classrooms.len() {
            return Err("教室を選んでください".into());
        }
        let c = self.competitors.get(comp).ok_or("競合が見つかりません")?;
        if c.acquired || star >= c.stars.len() {
            return Err("この講師はもういません".into());
        }
        if c.offer_cooldown > 0 {
            return Err(format!("{}とは交渉したばかりです（あと {} 週）", c.name, c.offer_cooldown));
        }
        let (salary, bonus) = self.star_offer_terms(comp, star, level);
        if self.cash < bonus {
            return Err("契約金を払う資金が足りません".into());
        }
        let chance = self.star_offer_chance(comp, level);
        let success = self.rng.chance(chance);
        let c = &mut self.competitors[comp];
        c.offer_cooldown = 12;
        if !success {
            c.grudge = c.grudge.max(12);
            return Ok(false);
        }

        let mut teacher = c.stars.remove(star);
        teacher.pay = salary;
        teacher.fatigue = 0.1;
        c.quality = (c.quality - 0.04).max(0.3);
        c.grudge = 52;
        // 同じエリアに教室があれば、慕う生徒が一部ついてくる
        let area = self.classrooms[ci].area;
        let mut followers = [0u32; 3];
        if let Some(b) = c.branch_mut(area) {
            for gi in 0..3 {
                let n = (b.students[gi] as f64 * 0.08) as u32;
                b.students[gi] -= n;
                followers[gi] = n;
            }
        }
        self.cash -= bonus;
        self.pending_hq.recruiting += bonus;
        for (gi, n) in followers.iter().enumerate() {
            for _ in 0..*n {
                let mut s = gen::make_student(&mut self.rng, &mut self.next_id, &self.schools, gi as u8 + 4, false);
                s.weeks_enrolled = 20;
                s.motivation = (s.motivation + 0.1).min(1.0);
                self.classrooms[ci].students.push(s);
            }
        }
        let room = &mut self.classrooms[ci];
        room.awareness = clamp01(room.awareness + 0.1);
        room.teachers.push(teacher);
        Ok(true)
    }
}

impl World {
    // ------------------------------------------------------------------
    // 教材の外販
    // ------------------------------------------------------------------

    /// 外販を始める（自社教材が必要）。
    pub fn start_material_sales(&mut self) -> Result<(), String> {
        if !self.materials.is_own() {
            return Err("外販するには自社教材が必要です".into());
        }
        if self.materials.sales.active {
            return Err("すでに外販しています".into());
        }
        if self.cash < materials::SALES_LAUNCH_COST {
            return Err("資金が足りません".into());
        }
        self.cash -= materials::SALES_LAUNCH_COST;
        self.pending_hq.launch += materials::SALES_LAUNCH_COST;
        let s = &mut self.materials.sales;
        s.active = true;
        s.started = Some(self.calendar.date());
        Ok(())
    }

    /// 外販をやめる（契約はすべて終わる）。
    pub fn stop_material_sales(&mut self) {
        let s = &mut self.materials.sales;
        s.active = false;
        s.contracts = 0;
    }

    pub fn set_material_fee(&mut self, fee: Yen) {
        self.materials.sales.fee = fee.clamp(5_000, 200_000);
    }

    pub fn set_sell_to_rivals(&mut self, on: bool) {
        self.materials.sales.sell_to_rivals = on;
    }

    // ------------------------------------------------------------------
    // 上場
    // ------------------------------------------------------------------

    /// その前の 52 週（53〜104 週前）の純利益。
    fn previous_year_profit(&self) -> Option<Yen> {
        if self.history.len() < 104 {
            return None;
        }
        Some(self.history.iter().rev().skip(52).take(52).map(|w| w.net()).sum())
    }

    /// 上場の条件と、いま満たしているか。
    pub fn ipo_requirements(&self) -> Vec<Requirement> {
        let profit = self.annual_profit();
        let prev = self.previous_year_profit();
        let padded = self.exam_results.last().map(|e| e.padded).unwrap_or(false) || self.padding_this_year;
        vec![
            Requirement {
                label: format!("教室を {} つ以上持つ", listing::REQ_CLASSROOMS),
                detail: format!("いま {} 教室", self.classrooms.len()),
                ok: self.classrooms.len() >= listing::REQ_CLASSROOMS,
            },
            Requirement {
                label: format!("直近 1 年の純利益が {} 以上", crate::format_yen(listing::REQ_PROFIT)),
                detail: format!("いま {}", crate::format_yen(profit)),
                ok: profit >= listing::REQ_PROFIT,
            },
            Requirement {
                label: "その前の 1 年も黒字".into(),
                detail: match prev {
                    Some(p) => crate::format_yen(p),
                    None => "まだ 2 年分の決算がありません".into(),
                },
                ok: prev.map(|p| p > 0).unwrap_or(false),
            },
            Requirement {
                label: "社会的な信頼が十分にある".into(),
                detail: if self.trust >= listing::REQ_TRUST {
                    "問題なし".into()
                } else {
                    "不祥事や強引な指導で信頼が下がっています".into()
                },
                ok: self.trust >= listing::REQ_TRUST,
            },
            Requirement {
                label: "合格実績の水増しをしていない".into(),
                detail: if padded {
                    "直近の入試で実績を上積みしています".into()
                } else {
                    "問題なし".into()
                },
                ok: !padded,
            },
            Requirement {
                label: "資金繰りに問題がない".into(),
                detail: if self.cash >= 0 { "問題なし".into() } else { "資金がマイナスです".into() },
                ok: self.cash >= 0,
            },
        ]
    }

    pub fn ipo_ready(&self) -> bool {
        self.ipo_requirements().iter().all(|r| r.ok)
    }

    /// 上場準備を始める（1 年かかり、費用は毎週かかる）。
    pub fn start_ipo(&mut self) -> Result<(), String> {
        if self.listing.status != ListingStatus::Private {
            return Err("すでに上場準備中か、上場しています".into());
        }
        if !self.ipo_ready() {
            return Err("上場の条件を満たしていません".into());
        }
        self.listing.status = ListingStatus::Preparing { weeks_left: listing::IPO_PREP_WEEKS };
        Ok(())
    }

    /// 上場準備をやめる（それまでの費用は戻らない）。
    pub fn cancel_ipo(&mut self) {
        if self.listing.is_preparing() {
            self.listing.status = ListingStatus::Private;
        }
    }

    /// 理論的な企業価値（株価の基準）：利益 × PER ＋ 純資産。
    /// PER はブランドと成長率で 6〜25 倍（上場している学習塾の水準）。
    pub fn fair_value(&self) -> Yen {
        let ttm = self.annual_profit();
        let growth = match self.previous_year_profit() {
            Some(prev) => ((ttm - prev) as f64 / (prev.abs().max(10_000_000)) as f64).clamp(-0.5, 1.0),
            None => 0.0,
        };
        let per = (8.0 + 10.0 * self.brand + 8.0 * growth).clamp(6.0, 25.0);
        let equity = self.cash + (self.property_assets() as f64 * 0.8) as Yen - self.debt();
        ((ttm.max(0) as f64 * per) as Yen + equity).max(0)
    }

    /// 1 株あたりの株価（未上場なら理論値）。
    pub fn share_price(&self) -> f64 {
        if self.listing.is_listed() {
            self.listing.price()
        } else {
            self.fair_value() as f64 / listing::FOUNDER_SHARES as f64
        }
    }

    /// 塾長の個人資産（持ち株の評価額）。
    pub fn personal_assets(&self) -> Yen {
        if self.listing.is_listed() {
            (self.listing.founder_shares as f64 * self.listing.price()) as Yen
        } else {
            self.valuation().max(0)
        }
    }

    /// 毎週の上場準備・株価の更新。
    pub(crate) fn advance_listing(&mut self, hq: &mut HqCosts, flows: &mut CapitalFlows, news: &mut Vec<String>) {
        match self.listing.status {
            ListingStatus::Private => {}
            ListingStatus::Preparing { weeks_left } => {
                self.cash -= listing::IPO_PREP_WEEKLY;
                hq.listing += listing::IPO_PREP_WEEKLY;
                if weeks_left > 1 {
                    self.listing.status = ListingStatus::Preparing { weeks_left: weeks_left - 1 };
                } else if self.ipo_ready() {
                    self.go_public(flows, news);
                } else {
                    self.listing.status = ListingStatus::Private;
                    self.listing.failed_attempts += 1;
                    news.insert(
                        0,
                        "上場審査に通りませんでした。条件を満たしてから、もう一度準備し直す必要があります".into(),
                    );
                }
            }
            ListingStatus::Listed => {
                self.cash -= listing::LISTED_WEEKLY;
                hq.listing += listing::LISTED_WEEKLY;
                // 時価総額は理論値にゆっくり近づき、市場の気まぐれで毎週ゆれる
                let target = self.fair_value() as f64;
                let cap = self.listing.market_cap as f64;
                let noise = 1.0 + self.rng.normal(0.0, 0.025);
                self.listing.market_cap = ((cap * 0.92 + target * 0.08) * noise).max(0.0) as Yen;
                self.listing.record_price();
            }
        }
    }

    fn go_public(&mut self, flows: &mut CapitalFlows, news: &mut Vec<String>) {
        let pre_money = self.fair_value().max(listing::REQ_PROFIT * 8) as f64;
        let fair_price = pre_money / listing::FOUNDER_SHARES as f64;
        let ipo_price = fair_price * listing::IPO_DISCOUNT;
        let raised = (listing::IPO_NEW_SHARES as f64 * ipo_price * (1.0 - listing::IPO_FEE)) as Yen;
        let l = &mut self.listing;
        l.status = ListingStatus::Listed;
        l.listed_on = Some(self.calendar.date());
        l.founder_shares = listing::FOUNDER_SHARES;
        l.shares = listing::FOUNDER_SHARES + listing::IPO_NEW_SHARES;
        l.ipo_price = ipo_price;
        l.raised = raised;
        // 初値は理論株価あたりから始まる
        l.market_cap = (fair_price * l.shares as f64) as Yen;
        l.price_history.clear();
        l.record_price();
        self.cash += raised;
        flows.equity += raised;
        self.trust = clamp01(self.trust + 0.05);
        news.insert(
            0,
            format!(
                "{}が株式を上場しました！公開価格 {} 円で {} を調達しました",
                self.company_name,
                ipo_price.round() as Yen,
                crate::format_yen(raised)
            ),
        );
    }
}
