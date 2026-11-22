//! マスターデータ。塾名・学校名・人名はすべて架空。

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
    pub style: &'static str,
    pub address: &'static str,
    pub brand: f64,
    pub quality: f64,
    pub care: f64,
    /// 学年別の月謝（4・5・6 年）
    pub fees: [i64; 3],
    pub students: [u32; 3],
    pub color: [u8; 3],
}

pub const COMPETITORS: &[CompetitorDef] = &[
    CompetitorDef {
        name: "英峰ゼミナール",
        style: "大手難関型",
        address: "駅前1-2-3",
        brand: 0.80,
        quality: 0.72,
        care: 0.35,
        fees: [30_000, 46_000, 62_000],
        students: [70, 75, 80],
        color: [0x4F, 0x8C, 0xF0],
    },
    CompetitorDef {
        name: "ひだまり進学教室",
        style: "面倒見型",
        address: "本町3-5-8",
        brand: 0.45,
        quality: 0.55,
        care: 0.80,
        fees: [24_000, 37_000, 50_000],
        students: [35, 35, 30],
        color: [0xF2, 0xB6, 0x3A],
    },
    CompetitorDef {
        name: "スタディリンク",
        style: "格安オンライン型",
        address: "オンライン",
        brand: 0.30,
        quality: 0.45,
        care: 0.25,
        fees: [12_000, 18_000, 26_000],
        students: [30, 25, 20],
        color: [0xA8, 0x6B, 0xE8],
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

pub const MANAGER_NAMES: &[&str] = &["真柴 恭介", "北条 さやか", "大野 健太郎", "早瀬 美咲"];
