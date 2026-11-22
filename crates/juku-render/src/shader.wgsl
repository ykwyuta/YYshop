// juku-render: 2D バッチ描画シェーダ
//
// すべての図形を「インスタンス化された四角形」として描き、フラグメントシェーダで
// SDF（符号付き距離場）を評価してアンチエイリアス付きの形を作る。
//
// kind:
//   0 = 角丸矩形（枠線つき、縦グラデーション可）
//   1 = グリフ（アトラスのアルファ）
//   2 = 円弧・リング（ドーナツグラフ）
//   3 = 線分（カプセル）
//   4 = 五芒星
//   5 = スプライト（画像アトラス）

struct Globals {
    // 画面サイズ（論理座標）
    screen: vec2<f32>,
    // 論理座標 → 物理ピクセルの倍率
    scale: f32,
    // 1 = 出力先が sRGB なので線形化する
    srgb: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_samp: sampler;
@group(0) @binding(3) var sprite_atlas: texture_2d<f32>;
@group(0) @binding(4) var sprite_samp: sampler;

struct Inst {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) clip: vec4<f32>,
    @location(3) uv: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) params2: vec4<f32>,
    @location(6) color2: vec4<f32>,
};

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) p: vec2<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) rect: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) clip: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) params2: vec4<f32>,
    @location(7) color2: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Inst) -> VOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[vi];
    let kind = inst.params.x;
    // 図形だけアンチエイリアス用に 1px 広げる
    var pad = 1.0 / globals.scale;
    if ((kind > 0.5 && kind < 1.5) || kind > 4.5) {
        pad = 0.0;
    }
    let origin = inst.rect.xy - vec2<f32>(pad, pad);
    let size = inst.rect.zw + vec2<f32>(2.0 * pad, 2.0 * pad);
    let p = origin + c * size;

    var out: VOut;
    let ndc = vec2<f32>(p.x / globals.screen.x * 2.0 - 1.0, 1.0 - p.y / globals.screen.y * 2.0);
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.p = p;
    out.tex = mix(inst.uv.xy, inst.uv.zw, c);
    out.rect = inst.rect;
    out.color = inst.color;
    out.clip = inst.clip;
    out.params = inst.params;
    out.params2 = inst.params2;
    out.color2 = inst.color2;
    return out;
}

fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// Inigo Quilez の sdStar5（y 上向き）
fn sd_star5(p_in: vec2<f32>, r: f32, rf: f32) -> f32 {
    let k1 = vec2<f32>(0.809016994375, -0.587785252292);
    let k2 = vec2<f32>(-k1.x, k1.y);
    var p = p_in;
    p.x = abs(p.x);
    p = p - 2.0 * max(dot(k1, p), 0.0) * k1;
    p = p - 2.0 * max(dot(k2, p), 0.0) * k2;
    p.x = abs(p.x);
    p.y = p.y - r;
    let ba = rf * vec2<f32>(-k1.y, k1.x) - vec2<f32>(0.0, 1.0);
    let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
    return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2, 2.2, 2.2));
}

fn finish(c: vec4<f32>) -> vec4<f32> {
    var rgb = c.rgb;
    if (globals.srgb > 0.5) {
        rgb = to_linear(rgb);
    }
    // 乗算済みアルファで出力
    return vec4<f32>(rgb * c.a, c.a);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let p = in.p;
    if (p.x < in.clip.x || p.y < in.clip.y || p.x > in.clip.z || p.y > in.clip.w) {
        discard;
    }
    let kind = in.params.x;
    let aa = 1.0 / globals.scale;

    // グリフ
    if (kind > 0.5 && kind < 1.5) {
        let a = textureSample(atlas_tex, atlas_samp, in.tex).r;
        return finish(vec4<f32>(in.color.rgb, in.color.a * a));
    }

    if (kind > 4.5) {
        let half = in.rect.zw * 0.5;
        let radius = min(in.params.y, min(half.x, half.y));
        let dist = sd_round_box(p - in.rect.xy - half, half, radius);
        let coverage = clamp(0.5 - dist / aa, 0.0, 1.0);
        let c = textureSampleLevel(sprite_atlas, sprite_samp, in.tex, 0.0);
        return finish(vec4<f32>(c.rgb, c.a * coverage));
    }

    // 円弧・リング
    if (kind > 1.5 && kind < 2.5) {
        let center = in.rect.xy + in.rect.zw * 0.5;
        let d = p - center;
        let r = length(d);
        let outer = in.rect.z * 0.5;
        let inner = in.params2.x;
        var dist = max(r - outer, inner - r);
        let start = in.params2.y;
        let end = in.params2.z;
        if (end - start < 6.2831) {
            // 0 = 真上、時計回り
            var a = atan2(d.x, -d.y);
            if (a < 0.0) {
                a = a + 6.2831853;
            }
            let da = min(a - start, end - a) * max(r, 1.0);
            dist = max(dist, -da);
        }
        let alpha = clamp(0.5 - dist / aa, 0.0, 1.0);
        return finish(vec4<f32>(in.color.rgb, in.color.a * alpha));
    }

    // 線分（カプセル）
    if (kind > 2.5 && kind < 3.5) {
        let a = in.params2.xy;
        let b = in.params2.zw;
        let pa = p - a;
        let ba = b - a;
        let h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.0001), 0.0, 1.0);
        let dist = length(pa - ba * h) - in.params.y * 0.5;
        let alpha = clamp(0.5 - dist / aa, 0.0, 1.0);
        return finish(vec4<f32>(in.color.rgb, in.color.a * alpha));
    }

    // 五芒星
    if (kind > 3.5 && kind < 4.5) {
        let center = in.rect.xy + in.rect.zw * 0.5;
        let q = p - center;
        let dist = sd_star5(vec2<f32>(q.x, -q.y), in.rect.z * 0.5, in.params2.x);
        let alpha = clamp(0.5 - dist / aa, 0.0, 1.0);
        return finish(vec4<f32>(in.color.rgb, in.color.a * alpha));
    }

    // 角丸矩形
    let half = in.rect.zw * 0.5;
    let center = in.rect.xy + half;
    let radius = min(in.params.y, min(half.x, half.y));
    let dist = sd_round_box(p - center, half, radius);
    let alpha = clamp(0.5 - dist / aa, 0.0, 1.0);
    var fill = in.color;
    // 縦グラデーション
    if (in.params.w > 0.5) {
        let t = clamp((p.y - in.rect.y) / max(in.rect.w, 1.0), 0.0, 1.0);
        fill = mix(in.color, in.color2, t);
    }
    let bw = in.params.z;
    if (bw > 0.0) {
        let inner = clamp(0.5 - (dist + bw) / aa, 0.0, 1.0);
        let border = vec4<f32>(in.color2.rgb, in.color2.a);
        var c = mix(border, fill, inner);
        if (in.params.w > 0.5) {
            c = mix(vec4<f32>(0.0, 0.0, 0.0, 0.0), fill, inner);
        }
        return finish(vec4<f32>(c.rgb, c.a * alpha));
    }
    return finish(vec4<f32>(fill.rgb, fill.a * alpha));
}
