//! マスターデータ。塾名・学校名・人名はすべて架空。

use crate::market::CompetitorStyle;
use crate::model::{CampaignKind, SchoolTier};

pub const SURNAMES: &[&str] = &[
    "佐藤",
    "鈴木",
    "高橋",
    "田中",
    "伊藤",
    "渡辺",
    "山本",
    "中村",
    "小林",
    "加藤",
    "吉田",
    "山田",
    "佐々木",
    "山口",
    "松本",
    "井上",
    "木村",
    "林",
    "清水",
    "山崎",
    "森",
    "池田",
    "橋本",
    "阿部",
    "石川",
    "前田",
    "藤田",
    "小川",
    "岡田",
    "後藤",
    "長谷川",
    "村上",
    "近藤",
    "石井",
    "坂本",
    "遠藤",
];

pub const GIVEN_NAMES: &[&str] = &[
    "陽翔", "蓮", "湊", "悠真", "大和", "樹", "颯太", "律", "朝陽", "蒼", "結翔", "陽向", "葵", "陽葵", "凛", "芽依",
    "結菜", "紬", "澪", "咲良", "美桜", "莉子", "心春", "杏", "結衣", "千尋", "遥", "奏", "光", "翼",
];

pub struct SchoolDef {
    pub name: &'static str,
    pub deviation: f64,
    pub tier: SchoolTier,
}

/// 架空の私立・公立中高一貫校。
pub const SCHOOLS: &[SchoolDef] = &[
    SchoolDef { name: "白鷺学院中", deviation: 68.0, tier: SchoolTier::Top },
    SchoolDef { name: "桐ヶ丘中", deviation: 66.0, tier: SchoolTier::Top },
    SchoolDef { name: "星稜女子中", deviation: 64.0, tier: SchoolTier::Top },
    SchoolDef { name: "明峰中", deviation: 61.0, tier: SchoolTier::Upper },
    SchoolDef { name: "青葉学園中", deviation: 59.0, tier: SchoolTier::Upper },
    SchoolDef { name: "聖マリア女学院中", deviation: 57.0, tier: SchoolTier::Upper },
    SchoolDef { name: "都立みなと中等教育学校", deviation: 58.0, tier: SchoolTier::Public },
    SchoolDef { name: "区立けやき中等学校", deviation: 54.0, tier: SchoolTier::Public },
    SchoolDef { name: "緑ヶ丘中", deviation: 53.0, tier: SchoolTier::Middle },
    SchoolDef { name: "東雲学園中", deviation: 50.0, tier: SchoolTier::Middle },
    SchoolDef { name: "若葉台中", deviation: 47.0, tier: SchoolTier::Middle },
    SchoolDef { name: "港南学院中", deviation: 44.0, tier: SchoolTier::Middle },
    SchoolDef { name: "さくら坂中", deviation: 40.0, tier: SchoolTier::Middle },
];

pub struct CompetitorDef {
    pub name: &'static str,
    pub style: CompetitorStyle,
    pub brand: f64,
    pub quality: f64,
    pub care: f64,
    /// 学年別の月謝（4・5・6 年）
    pub fees: [i64; 3],
    pub color: [u8; 3],
    /// 出店しているエリアと、そこでの在籍数（4・5・6 年）
    pub branches: &'static [(usize, [u32; 3])],
}

pub const COMPETITORS: &[CompetitorDef] = &[
    CompetitorDef {
        name: "英峰ゼミナール",
        style: CompetitorStyle::Elite,
        brand: 0.80,
        quality: 0.72,
        care: 0.35,
        fees: [30_000, 46_000, 62_000],
        color: [0x4F, 0x8C, 0xF0],
        branches: &[(0, [70, 75, 80]), (1, [90, 95, 100]), (4, [50, 55, 60])],
    },
    CompetitorDef {
        name: "ひだまり進学教室",
        style: CompetitorStyle::Caring,
        brand: 0.45,
        quality: 0.55,
        care: 0.80,
        fees: [24_000, 37_000, 50_000],
        color: [0xF2, 0xB6, 0x3A],
        branches: &[(0, [35, 35, 30]), (2, [40, 38, 32])],
    },
    CompetitorDef {
        name: "スタディリンク",
        style: CompetitorStyle::Online,
        brand: 0.30,
        quality: 0.45,
        care: 0.25,
        fees: [12_000, 18_000, 26_000],
        color: [0xA8, 0x6B, 0xE8],
        branches: &[(0, [30, 25, 20]), (1, [30, 25, 20]), (2, [20, 18, 15]), (3, [15, 12, 10]), (4, [25, 22, 18])],
    },
    CompetitorDef {
        name: "青嵐アカデミー",
        style: CompetitorStyle::Individual,
        brand: 0.40,
        quality: 0.60,
        care: 0.65,
        fees: [32_000, 45_000, 58_000],
        color: [0x41, 0xC6, 0xC0],
        branches: &[(2, [20, 22, 25]), (4, [25, 28, 30])],
    },
    CompetitorDef {
        name: "小林算数教室",
        style: CompetitorStyle::Independent,
        brand: 0.25,
        quality: 0.66,
        care: 0.80,
        fees: [22_000, 33_000, 45_000],
        color: [0xB0, 0x8D, 0x6B],
        branches: &[(0, [7, 7, 6])],
    },
    CompetitorDef {
        name: "まなび舎 森田塾",
        style: CompetitorStyle::Independent,
        brand: 0.22,
        quality: 0.60,
        care: 0.85,
        fees: [20_000, 31_000, 42_000],
        color: [0x8F, 0xB0, 0x6B],
        branches: &[(3, [9, 8, 7])],
    },
    CompetitorDef {
        name: "進学ゼミ・ブライト",
        style: CompetitorStyle::SmallChain,
        brand: 0.32,
        quality: 0.55,
        care: 0.55,
        fees: [24_000, 36_000, 50_000],
        color: [0xE0, 0x7A, 0x5F],
        branches: &[(2, [18, 16, 14]), (4, [16, 15, 13])],
    },
    CompetitorDef {
        name: "東雲アカデミア",
        style: CompetitorStyle::Independent,
        brand: 0.28,
        quality: 0.70,
        care: 0.70,
        fees: [26_000, 40_000, 54_000],
        color: [0x9A, 0x9A, 0xC8],
        branches: &[(4, [8, 9, 9])],
    },
];

pub struct AreaDef {
    pub name: &'static str,
    pub address: &'static str,
    pub description: &'static str,
    /// 市場規模（入塾希望者数の倍率）
    pub pool: f64,
    /// 週あたりの家賃
    pub rent: i64,
    /// 開校費（敷金・内装・備品）
    pub open_cost: i64,
    /// 教室の物件を買い取る場合の価格
    pub property_price: i64,
    /// 市場規模の年間の伸び（新興エリアは伸び、高齢化エリアは縮む）
    pub pool_growth: f64,
}

pub const AREAS: &[AreaDef] = &[
    AreaDef {
        name: "けやき台",
        address: "けやき台2-5-33",
        description: "創業の地。大手と面倒見型がひしめく",
        pool: 1.0,
        rent: 120_000,
        open_cost: 0,
        property_price: 95_000_000,
        pool_growth: 0.0,
    },
    AreaDef {
        name: "桜ヶ丘",
        address: "桜ヶ丘駅前1-8",
        description: "教育熱心な家庭が多い人気エリア。英峰ゼミナールの牙城",
        pool: 1.25,
        rent: 170_000,
        open_cost: 9_000_000,
        property_price: 135_000_000,
        pool_growth: 0.01,
    },
    AreaDef {
        name: "港南",
        address: "港南3-2-11",
        description: "家賃が安く、面倒見型と個別指導が強い",
        pool: 0.8,
        rent: 90_000,
        open_cost: 6_000_000,
        property_price: 70_000_000,
        pool_growth: 0.0,
    },
    AreaDef {
        name: "緑町",
        address: "緑町5-1-4",
        description: "大手が手薄な住宅地。市場は小さく、少しずつ縮んでいる",
        pool: 0.7,
        rent: 85_000,
        open_cost: 5_500_000,
        property_price: 64_000_000,
        pool_growth: -0.02,
    },
    AreaDef {
        name: "東雲",
        address: "東雲ベイ2-3",
        description: "タワーマンションが増える新興エリア。市場が毎年 6% ずつ伸びる",
        pool: 1.15,
        rent: 150_000,
        open_cost: 8_000_000,
        property_price: 120_000_000,
        pool_growth: 0.06,
    },
];

/// 学年別の市場標準月謝（4・5・6 年）。
pub const REFERENCE_FEES: [i64; 3] = [25_000, 40_000, 55_000];

pub struct CampaignDef {
    pub kind: CampaignKind,
    pub name: &'static str,
    pub description: &'static str,
}

pub const CAMPAIGNS: &[CampaignDef] = &[
    CampaignDef {
        kind: CampaignKind::FreeTrial,
        name: "無料体験授業",
        description: "体験した親子の入塾率が上がる。体験生 1 人あたり講師コストがかかる",
    },
    CampaignDef {
        kind: CampaignKind::ResultsFlyer,
        name: "合格実績チラシ",
        description: "地域の認知度を上げる。合格実績が多いほどよく効く",
    },
    CampaignDef {
        kind: CampaignKind::ParentSeminar,
        name: "保護者説明会",
        description: "教育熱心な家庭に響く。面倒見の評価も上がる",
    },
    CampaignDef {
        kind: CampaignKind::StudyRoom,
        name: "自習室開放",
        description: "生徒の学力と満足度が上がる。家賃と監督費がかかる",
    },
    CampaignDef {
        kind: CampaignKind::SiblingDiscount,
        name: "兄弟・紹介割引",
        description: "退塾が減り紹介が増える。授業料の一部を割り引く",
    },
    CampaignDef {
        kind: CampaignKind::Scholarship,
        name: "特待生制度",
        description: "成績上位の子を授業料免除で迎える。合格実績に効くが不公平感も",
    },
    CampaignDef {
        kind: CampaignKind::SafetyMail, name: "入退室メール", description: "保護者の安心につながる"
    },
    CampaignDef {
        kind: CampaignKind::WebAds, name: "Web 広告", description: "若い保護者層への認知を広げる"
    },
];

pub const MANAGER_NAMES: &[&str] = &[
    "真柴 恭介",
    "北条 さやか",
    "大野 健太郎",
    "早瀬 美咲",
    "柏木 涼",
    "三上 理沙",
    "相沢 拓海",
    "桐島 奈央",
    "久保田 誠",
    "白石 遥香",
];
