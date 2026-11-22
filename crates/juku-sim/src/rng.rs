//! 決定論的な乱数生成器（PCG32）。
//!
//! Windows と WebAssembly で同じシードから同じ結果を得るため、外部の乱数クレートは使わない。

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
    inc: u64,
}

const MULT: u64 = 6_364_136_223_846_793_005;

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng { state: 0, inc: (seed << 1) | 1 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed ^ 0x9E37_79B9_7F4A_7C15);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// [0, 1) の一様乱数。
    pub fn f64(&mut self) -> f64 {
        (self.next_u32() as f64) / 4_294_967_296.0
    }

    /// [lo, hi) の一様乱数。
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// [0, n) の整数。
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        ((self.next_u32() as u64 * n as u64) >> 32) as usize
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// 近似正規分布（一様乱数 4 個の和）。超越関数を使わないので環境差が出ない。
    pub fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        let s = self.f64() + self.f64() + self.f64() + self.f64() - 2.0;
        // 一様分布 4 個の和の分散は 4/12 = 1/3
        mean + sd * s * 1.732_050_807_568_877_2
    }

    /// 期待値 `x` の非負整数（整数部 + 端数を確率で切り上げ）。
    pub fn round_stochastic(&mut self, x: f64) -> u32 {
        if x <= 0.0 {
            return 0;
        }
        let base = x as u32;
        let frac = x - base as f64;
        base + u32::from(self.chance(frac))
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn f64_in_range() {
        let mut r = Rng::new(1);
        for _ in 0..10_000 {
            let x = r.f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn normal_mean_is_close() {
        let mut r = Rng::new(7);
        let n = 20_000;
        let mean: f64 = (0..n).map(|_| r.normal(50.0, 10.0)).sum::<f64>() / n as f64;
        assert!((mean - 50.0).abs() < 0.5, "mean = {mean}");
    }
}
