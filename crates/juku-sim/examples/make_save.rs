//! 自動プレイで進めたセーブデータを作る（画面の確認用）。
//!   cargo run -p juku-sim --example make_save --release -- [年数] [シード] > save.b64
//! 出力は localStorage の `yyshop.autosave` にそのまま入れられる Base64。
use juku_sim::bot::{act_week, Strategy};
use juku_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let years: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let mut w = World::new(seed);
    for week in 0..years * 52 {
        act_week(&mut w, Strategy::Growth, week);
        w.step_week();
    }
    // 判断待ちや未読の合格発表は片付けておく
    w.pending_event = None;
    w.unseen_exam_result = false;
    eprintln!(
        "教室 {} / 上場 {} / 外販 {} 塾 / 資金 {}",
        w.classrooms.len(),
        w.listing.is_listed(),
        w.materials.sales.contracts,
        juku_sim::format_yen(w.cash)
    );
    let bytes = postcard::to_allocvec(&w).expect("serialize");
    print!("{}", base64(&bytes));
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
