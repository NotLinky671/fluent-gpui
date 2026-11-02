//! Code generation for fluent-gpui.
//!
//! `cargo xtask gen-tokens [--check]`
//!
//! Reads `vendor/tokens.json` (produced by `scripts/snapshot-tokens.mjs` from
//! a pinned `@fluentui/tokens`) and writes
//! `crates/fluent-tokens/src/generated.rs`. With `--check` it regenerates into
//! memory and fails when the file on disk differs, which is what CI runs.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

const SOURCE: &str = "vendor/tokens.json";
const OUTPUT: &str = "crates/fluent-tokens/src/generated.rs";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    match args.first().map(String::as_str) {
        Some("gen-tokens") => {
            if let Err(error) = gen_tokens(check) {
                eprintln!("gen-tokens failed: {error}");
                std::process::exit(1);
            }
        }
        Some("gen-icons") => {
            if let Err(error) = gen_icons(check) {
                eprintln!("gen-icons failed: {error}");
                std::process::exit(1);
            }
        }
        Some("tasks") => {
            if let Err(error) = tasks(check) {
                eprintln!("tasks failed: {error}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo xtask gen-tokens|gen-icons|tasks [--check]");
            std::process::exit(2);
        }
    }
}

fn gen_tokens(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let raw = std::fs::read_to_string(root.join(SOURCE))?;
    let snapshot: serde_json::Value = serde_json::from_str(&raw)?;
    let generated = render(&snapshot)?;

    let out = root.join(OUTPUT);
    if check {
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        if current != generated {
            eprintln!("{OUTPUT} is out of date; run `cargo xtask gen-tokens`");
            std::process::exit(1);
        }
        println!("{OUTPUT} is up to date");
        return Ok(());
    }

    std::fs::write(&out, &generated)?;
    println!("wrote {OUTPUT} ({} bytes)", generated.len());
    Ok(())
}

fn workspace_root() -> PathBuf {
    // xtask runs from the workspace root via the cargo alias.
    std::env::current_dir().expect("cwd")
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Color,
    FontSize,
    LineHeight,
    FontWeight,
    FontFamily,
    SpacingHorizontal,
    SpacingVertical,
    Radius,
    StrokeWidth,
    Duration,
    Curve,
    Shadow,
}

impl Group {
    fn of(token: &str) -> Option<(Group, &str)> {
        const PREFIXES: &[(&str, Group)] = &[
            ("color", Group::Color),
            ("fontSize", Group::FontSize),
            ("lineHeight", Group::LineHeight),
            ("fontWeight", Group::FontWeight),
            ("fontFamily", Group::FontFamily),
            ("spacingHorizontal", Group::SpacingHorizontal),
            ("spacingVertical", Group::SpacingVertical),
            ("borderRadius", Group::Radius),
            ("strokeWidth", Group::StrokeWidth),
            ("duration", Group::Duration),
            ("curve", Group::Curve),
            ("shadow", Group::Shadow),
        ];
        PREFIXES
            .iter()
            .find_map(|(prefix, group)| token.strip_prefix(prefix).map(|rest| (*group, rest)))
    }

    fn struct_name(self) -> &'static str {
        match self {
            Group::Color => "ColorTokens",
            Group::FontSize => "FontSizeTokens",
            Group::LineHeight => "LineHeightTokens",
            Group::FontWeight => "FontWeightTokens",
            Group::FontFamily => "FontFamilyTokens",
            Group::SpacingHorizontal | Group::SpacingVertical => "SpacingScale",
            Group::Radius => "RadiusTokens",
            Group::StrokeWidth => "StrokeWidthTokens",
            Group::Duration => "DurationTokens",
            Group::Curve => "CurveTokens",
            Group::Shadow => "ShadowTokens",
        }
    }

    /// `SharedString` and `BoxShadow` are not `Copy`, so those groups are not.
    fn is_copy(self) -> bool {
        !matches!(self, Group::FontFamily | Group::Shadow)
    }

    fn rust_type(self) -> &'static str {
        match self {
            Group::Color => "Hsla",
            Group::FontWeight => "FontWeight",
            Group::FontFamily => "SharedString",
            Group::Duration => "Duration",
            Group::Curve => "CubicBezier",
            Group::Shadow => "[BoxShadow; 2]",
            _ => "Pixels",
        }
    }
}

fn field_name(group: Group, rest: &str) -> String {
    match group {
        // shadow2 -> s2, shadow2Brand -> s2_brand
        Group::Shadow => {
            let (number, brand) = match rest.strip_suffix("Brand") {
                Some(number) => (number, true),
                None => (rest, false),
            };
            if brand {
                format!("s{number}_brand")
            } else {
                format!("s{number}")
            }
        }
        _ => {
            let snake = snake_case(rest);
            if snake.starts_with(|c: char| c.is_ascii_digit()) {
                format!("r{snake}")
            } else {
                snake
            }
        }
    }
}

fn snake_case(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 4);
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                Some(p) if p.is_ascii_lowercase() || p.is_ascii_digit() => true,
                Some(p) if p.is_ascii_uppercase() => next.is_some_and(|n| n.is_ascii_lowercase()),
                _ => false,
            };
            if boundary {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_digit() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            if prev.is_some_and(|p| p.is_ascii_alphabetic()) {
                out.push('_');
            }
            out.push(ch);
        } else {
            out.push(ch);
        }
    }
    out
}

/// Field layout of `Tokens`, in declaration order.
const TOKENS_FIELDS: &[(&str, &str)] = &[
    ("color", "ColorTokens"),
    ("font_size", "FontSizeTokens"),
    ("line_height", "LineHeightTokens"),
    ("font_weight", "FontWeightTokens"),
    ("font_family", "FontFamilyTokens"),
    ("spacing", "SpacingTokens"),
    ("radius", "RadiusTokens"),
    ("stroke_width", "StrokeWidthTokens"),
    ("duration", "DurationTokens"),
    ("curve", "CurveTokens"),
    ("shadow", "ShadowTokens"),
];

fn render(snapshot: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let source = snapshot["source"].as_str().unwrap_or("unknown");
    let themes = snapshot["themes"].as_object().ok_or("missing themes")?;
    let brands = snapshot["brands"].as_object().ok_or("missing brands")?;
    let names: Vec<String> = snapshot["tokenNames"]
        .as_array()
        .ok_or("missing tokenNames")?
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();

    let mut by_group: BTreeMap<Group, Vec<(String, String)>> = BTreeMap::new();
    for name in &names {
        let (group, rest) = Group::of(name).ok_or_else(|| format!("unclassified token {name}"))?;
        by_group
            .entry(group)
            .or_default()
            .push((field_name(group, rest), name.clone()));
    }

    // Horizontal and vertical spacing must expose identical field names; the
    // generator would otherwise emit two different SpacingScale shapes.
    let horizontal: Vec<String> = by_group[&Group::SpacingHorizontal]
        .iter()
        .map(|(field, _)| field.clone())
        .collect();
    let vertical: Vec<String> = by_group[&Group::SpacingVertical]
        .iter()
        .map(|(field, _)| field.clone())
        .collect();
    if horizontal != vertical {
        return Err("spacingHorizontal and spacingVertical field names differ".into());
    }

    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo xtask gen-tokens` from {SOURCE} ({source})."
    )?;
    writeln!(out, "// Do not edit by hand: run the generator instead.")?;
    writeln!(out)?;
    writeln!(out, "use std::time::Duration;")?;
    writeln!(out)?;
    writeln!(
        out,
        "use gpui::{{BoxShadow, FontWeight, Hsla, Pixels, SharedString, point, px, rgba, transparent_black}};"
    )?;
    writeln!(out)?;
    writeln!(out, "use crate::{{BrandVariants, CubicBezier}};")?;
    writeln!(out)?;
    writeln!(out, "/// Number of Fluent tokens in this snapshot.")?;
    writeln!(out, "pub const TOKEN_COUNT: usize = {};", names.len())?;
    writeln!(out)?;

    // Tokens
    writeln!(out, "/// All Fluent tokens for one appearance.")?;
    writeln!(out, "#[derive(Clone, Debug, PartialEq)]")?;
    writeln!(out, "pub struct Tokens {{")?;
    for (field, struct_name) in TOKENS_FIELDS {
        writeln!(out, "    pub {field}: {struct_name},")?;
    }
    writeln!(out, "}}")?;
    writeln!(out)?;

    // SpacingTokens wraps two identical scales.
    writeln!(out, "/// `spacingHorizontal*` and `spacingVertical*`.")?;
    writeln!(out, "#[derive(Clone, Copy, Debug, PartialEq)]")?;
    writeln!(out, "pub struct SpacingTokens {{")?;
    writeln!(out, "    pub horizontal: SpacingScale,")?;
    writeln!(out, "    pub vertical: SpacingScale,")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    // one struct per remaining group
    for group in [
        Group::Color,
        Group::FontSize,
        Group::LineHeight,
        Group::FontWeight,
        Group::FontFamily,
        Group::Radius,
        Group::StrokeWidth,
        Group::Duration,
        Group::Curve,
        Group::Shadow,
    ] {
        let fields = by_group
            .get(&group)
            .ok_or_else(|| format!("no fields for {:?}", group.struct_name()))?;
        let derives = if group.is_copy() {
            "#[derive(Clone, Copy, Debug, PartialEq)]"
        } else {
            "#[derive(Clone, Debug, PartialEq)]"
        };
        writeln!(out, "{derives}")?;
        writeln!(out, "pub struct {} {{", group.struct_name())?;
        for (field, _) in fields {
            writeln!(out, "    pub {field}: {},", group.rust_type())?;
        }
        writeln!(out, "}}")?;
        writeln!(out)?;
    }

    // SpacingScale (shared shape)
    writeln!(out, "#[derive(Clone, Copy, Debug, PartialEq)]")?;
    writeln!(out, "pub struct SpacingScale {{")?;
    for field in &horizontal {
        writeln!(out, "    pub {field}: Pixels,")?;
    }
    writeln!(out, "}}")?;
    writeln!(out)?;

    // constructors
    writeln!(out, "impl Tokens {{")?;
    for (theme_key, fn_name, ramp) in [
        ("webLight", "web_light", Some("WEB")),
        ("webDark", "web_dark", Some("WEB")),
        ("teamsLight", "teams_light", Some("TEAMS")),
        ("teamsDark", "teams_dark", Some("TEAMS")),
        ("webHighContrast", "high_contrast", None),
    ] {
        let theme = themes
            .get(theme_key)
            .ok_or_else(|| format!("missing theme {theme_key}"))?
            .as_object()
            .ok_or("theme is not an object")?;
        let brand_map = brands
            .get(theme_key)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        writeln!(out, "    /// Fluent `{theme_key}` theme.")?;
        match ramp {
            Some(ramp) => {
                writeln!(
                    out,
                    "    pub fn {fn_name}_with(brand: &BrandVariants) -> Self {{"
                )?;
                writeln!(out, "        Self {{")?;
                for (field, struct_name) in TOKENS_FIELDS {
                    writeln!(out, "            {field}: {struct_name} {{")?;
                    match *struct_name {
                        "SpacingTokens" => {
                            writeln!(out, "                horizontal: SpacingScale {{")?;
                            emit_values(
                                &mut out,
                                &by_group[&Group::SpacingHorizontal],
                                theme,
                                &brand_map,
                                Group::SpacingHorizontal,
                            )?;
                            writeln!(out, "                }},")?;
                            writeln!(out, "                vertical: SpacingScale {{")?;
                            emit_values(
                                &mut out,
                                &by_group[&Group::SpacingVertical],
                                theme,
                                &brand_map,
                                Group::SpacingVertical,
                            )?;
                            writeln!(out, "                }},")?;
                        }
                        other => {
                            let group = group_of_struct(other);
                            emit_values(&mut out, &by_group[&group], theme, &brand_map, group)?;
                        }
                    }
                    writeln!(out, "            }},")?;
                }
                writeln!(out, "        }}")?;
                writeln!(out, "    }}")?;
                writeln!(out)?;
                writeln!(
                    out,
                    "    /// Fluent `{theme_key}` theme on the built-in ramp."
                )?;
                writeln!(out, "    pub fn {fn_name}() -> Self {{")?;
                writeln!(out, "        Self::{fn_name}_with(&BrandVariants::{ramp})")?;
                writeln!(out, "    }}")?;
            }
            None => {
                writeln!(out, "    pub fn {fn_name}() -> Self {{")?;
                writeln!(out, "        Self {{")?;
                for (field, struct_name) in TOKENS_FIELDS {
                    writeln!(out, "            {field}: {struct_name} {{")?;
                    match *struct_name {
                        "SpacingTokens" => {
                            writeln!(out, "                horizontal: SpacingScale {{")?;
                            emit_values(
                                &mut out,
                                &by_group[&Group::SpacingHorizontal],
                                theme,
                                &brand_map,
                                Group::SpacingHorizontal,
                            )?;
                            writeln!(out, "                }},")?;
                            writeln!(out, "                vertical: SpacingScale {{")?;
                            emit_values(
                                &mut out,
                                &by_group[&Group::SpacingVertical],
                                theme,
                                &brand_map,
                                Group::SpacingVertical,
                            )?;
                            writeln!(out, "                }},")?;
                        }
                        other => {
                            let group = group_of_struct(other);
                            emit_values(&mut out, &by_group[&group], theme, &brand_map, group)?;
                        }
                    }
                    writeln!(out, "            }},")?;
                }
                writeln!(out, "        }}")?;
                writeln!(out, "    }}")?;
            }
        }
        writeln!(out)?;
    }
    writeln!(out, "}}")?;

    writeln!(out)?;
    writeln!(out, "fn hex(value: u32) -> Hsla {{")?;
    writeln!(out, "    Hsla::from(rgba((value << 8) | 0xff))")?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    writeln!(
        out,
        "fn layer(x: f32, y: f32, blur: f32, color: Hsla) -> BoxShadow {{"
    )?;
    writeln!(out, "    BoxShadow {{")?;
    writeln!(out, "        color,")?;
    writeln!(out, "        offset: point(px(x), px(y)),")?;
    writeln!(out, "        blur_radius: px(blur),")?;
    writeln!(out, "        spread_radius: px(0.),")?;
    writeln!(out, "        inset: false,")?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;

    Ok(out)
}

fn emit_values(
    out: &mut String,
    fields: &[(String, String)],
    theme: &serde_json::Map<String, serde_json::Value>,
    brands: &serde_json::Map<String, serde_json::Value>,
    group: Group,
) -> Result<(), Box<dyn std::error::Error>> {
    for (field, token) in fields {
        let value = theme
            .get(token)
            .ok_or_else(|| format!("theme missing {token}"))?;
        let expr = if let Some(stop) = brands.get(token) {
            format!("brand.stop({})", stop.as_u64().unwrap_or(0))
        } else {
            value_expr(group, value)?
        };
        writeln!(out, "                    {field}: {expr},")?;
    }
    Ok(())
}

fn group_of_struct(struct_name: &str) -> Group {
    match struct_name {
        "ColorTokens" => Group::Color,
        "FontSizeTokens" => Group::FontSize,
        "LineHeightTokens" => Group::LineHeight,
        "FontWeightTokens" => Group::FontWeight,
        "FontFamilyTokens" => Group::FontFamily,
        "RadiusTokens" => Group::Radius,
        "StrokeWidthTokens" => Group::StrokeWidth,
        "DurationTokens" => Group::Duration,
        "CurveTokens" => Group::Curve,
        "ShadowTokens" => Group::Shadow,
        other => panic!("unknown struct {other}"),
    }
}
fn value_expr(
    group: Group,
    value: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    match group {
        Group::Color => color_expr(value.as_str().ok_or("color is not a string")?),
        Group::FontWeight => {
            let weight = value.as_f64().ok_or("weight is not a number")?;
            let name = match weight as u32 {
                400 => "NORMAL",
                500 => "MEDIUM",
                600 => "SEMIBOLD",
                700 => "BOLD",
                other => return Err(format!("unknown font weight {other}").into()),
            };
            Ok(format!("FontWeight::{name}"))
        }
        Group::FontFamily => {
            let family = value.as_str().ok_or("family is not a string")?;
            Ok(format!("SharedString::from({family:?})"))
        }
        Group::Duration => {
            let ms = value
                .as_str()
                .and_then(|s| s.strip_suffix("ms"))
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| format!("bad duration {value}"))?;
            Ok(format!("Duration::from_millis({ms})"))
        }
        Group::Curve => {
            let text = value.as_str().ok_or("curve is not a string")?;
            let inner = text
                .strip_prefix("cubic-bezier(")
                .and_then(|s| s.strip_suffix(')'))
                .ok_or_else(|| format!("bad curve {text}"))?;
            let nums: Vec<f32> = inner
                .split(',')
                .map(|part| part.trim().parse::<f32>())
                .collect::<Result<_, _>>()?;
            if nums.len() != 4 {
                return Err(format!("curve needs 4 points: {text}").into());
            }
            Ok(format!(
                "CubicBezier::new({}, {}, {}, {})",
                num(nums[0]),
                num(nums[1]),
                num(nums[2]),
                num(nums[3])
            ))
        }
        Group::Shadow => {
            let text = value.as_str().ok_or("shadow is not a string")?;
            let layers = split_shadow_layers(text)?;
            if layers.len() != 2 {
                return Err(
                    format!("expected 2 shadow layers, got {}: {text}", layers.len()).into(),
                );
            }
            let mut exprs = Vec::new();
            for layer in layers {
                exprs.push(shadow_layer_expr(&layer)?);
            }
            Ok(format!("[{}, {}]", exprs[0], exprs[1]))
        }
        _ => {
            let text = value.as_str().ok_or("length is not a string")?;
            let number = text
                .strip_suffix("px")
                .unwrap_or(text)
                .parse::<f32>()
                .map_err(|_| format!("bad length {text}"))?;
            Ok(format!("px({})", num(number)))
        }
    }
}

fn color_expr(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = text.trim();
    if text == "transparent" {
        return Ok("transparent_black()".to_string());
    }
    if let Some(hex) = text.strip_prefix('#') {
        return match hex.len() {
            6 => Ok(format!("hex(0x{})", hex.to_ascii_lowercase())),
            8 => Ok(format!("Hsla::from(rgba(0x{}))", hex.to_ascii_lowercase())),
            _ => Err(format!("bad hex color {text}").into()),
        };
    }
    if let Some(inner) = text.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            return Err(format!("bad rgba {text}").into());
        }
        let r: u8 = parts[0].parse()?;
        let g: u8 = parts[1].parse()?;
        let b: u8 = parts[2].parse()?;
        let a: f32 = parts[3].parse()?;
        return Ok(format!(
            "Hsla::from(rgba(0x{r:02x}{g:02x}{b:02x}ff)).alpha({})",
            num(a)
        ));
    }
    if let Some(inner) = text.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err(format!("bad rgb {text}").into());
        }
        let r: u8 = parts[0].parse()?;
        let g: u8 = parts[1].parse()?;
        let b: u8 = parts[2].parse()?;
        return Ok(format!("hex(0x{r:02x}{g:02x}{b:02x})"));
    }
    Err(format!("unhandled color {text}").into())
}

fn split_shadow_layers(text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut layers = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                layers.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    layers.push(current);
    Ok(layers
        .into_iter()
        .map(|layer| layer.trim().to_string())
        .filter(|layer| !layer.is_empty())
        .collect())
}

fn shadow_layer_expr(layer: &str) -> Result<String, Box<dyn std::error::Error>> {
    // "0 1px 2px rgba(0,0,0,0.14)"
    let mut parts = layer.splitn(4, ' ');
    let x: f32 = parts
        .next()
        .ok_or("shadow x")?
        .trim_end_matches("px")
        .parse()?;
    let y: f32 = parts
        .next()
        .ok_or("shadow y")?
        .trim_end_matches("px")
        .parse()?;
    let blur: f32 = parts
        .next()
        .ok_or("shadow blur")?
        .trim_end_matches("px")
        .parse()?;
    let color = parts.next().ok_or("shadow color")?;
    Ok(format!(
        "layer({}, {}, {}, {})",
        num(x),
        num(y),
        num(blur),
        color_expr(color)?
    ))
}

/// Formats a float without a trailing `.0` when it is integral.
fn num(value: f32) -> String {
    if (value - value.round()).abs() < f32::EPSILON {
        format!("{:.1}", value)
    } else {
        format!("{value}")
    }
}
const ICON_SOURCE: &str = "vendor/icons.json";
const ICON_OUTPUT: &str = "crates/fluent-icons/src/generated.rs";

fn gen_icons(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let raw = std::fs::read_to_string(root.join(ICON_SOURCE))?;
    let snapshot: serde_json::Value = serde_json::from_str(&raw)?;
    let generated = render_icons(&snapshot)?;

    let out = root.join(ICON_OUTPUT);
    if check {
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        if current != generated {
            eprintln!("{ICON_OUTPUT} is out of date; run `cargo xtask gen-icons`");
            std::process::exit(1);
        }
        println!("{ICON_OUTPUT} is up to date");
        return Ok(());
    }
    std::fs::write(&out, &generated)?;
    println!("wrote {ICON_OUTPUT} ({} bytes)", generated.len());
    Ok(())
}

fn render_icons(snapshot: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let source = snapshot["source"].as_str().unwrap_or("unknown");
    let icons = snapshot["icons"].as_object().ok_or("missing icons")?;

    let mut names: Vec<&String> = icons.keys().collect();
    names.sort();

    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo xtask gen-icons` from {ICON_SOURCE} ({source})."
    )?;
    writeln!(out, "// Do not edit by hand: run the generator instead.")?;
    writeln!(out)?;
    writeln!(out, "use crate::IconStyle;")?;
    writeln!(out)?;
    writeln!(
        out,
        "/// Fluent System Icons (MIT) embedded in the library."
    )?;
    writeln!(out, "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")?;
    writeln!(out, "pub enum FluentIcon {{")?;
    for name in &names {
        writeln!(out, "    {},", variant_name(name))?;
    }
    writeln!(out, "}}")?;
    writeln!(out)?;

    writeln!(out, "impl FluentIcon {{")?;
    writeln!(out, "    /// Every embedded icon, sorted by name.")?;
    writeln!(out, "    pub const ALL: &'static [FluentIcon] = &[")?;
    for name in &names {
        writeln!(out, "        FluentIcon::{},", variant_name(name))?;
    }
    writeln!(out, "    ];")?;
    writeln!(out)?;
    writeln!(out, "    /// The snake_case name used by the snapshot.")?;
    writeln!(out, "    pub fn name(self) -> &'static str {{")?;
    writeln!(out, "        match self {{")?;
    for name in &names {
        writeln!(
            out,
            "            FluentIcon::{} => {name:?},",
            variant_name(name)
        )?;
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    writeln!(out, "    /// Raw SVG bytes for one style.")?;
    writeln!(
        out,
        "    pub fn svg(self, style: IconStyle) -> &'static [u8] {{"
    )?;
    writeln!(out, "        match (self, style) {{")?;
    for name in &names {
        let styles = icons[*name].as_object().ok_or("icon is not an object")?;
        for style in ["regular", "filled"] {
            let svg = styles
                .get(style)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("{name} missing {style}"))?;
            writeln!(
                out,
                "            (FluentIcon::{}, IconStyle::{}) => {}.as_slice(),",
                variant_name(name),
                style_variant(style),
                raw_byte_string(svg)
            )?;
        }
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;
    Ok(out)
}

fn variant_name(snake: &str) -> String {
    snake
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn style_variant(style: &str) -> &'static str {
    match style {
        "regular" => "Regular",
        "filled" => "Filled",
        other => panic!("unknown icon style {other}"),
    }
}

/// Emits an SVG as a raw byte string, widening the hash count when the payload
/// contains the closing delimiter.
fn raw_byte_string(svg: &str) -> String {
    let mut hashes = 1;
    while svg.contains(&format!("\"{}", "#".repeat(hashes))) {
        hashes += 1;
    }
    let fence = "#".repeat(hashes);
    format!("br{fence}\"{svg}\"{fence}")
}

const TASK_ROOT: &str = "task";
const TASK_INDEX: &str = "task/_index.md";

#[derive(Debug, Clone)]
struct Task {
    id: String,
    gate: String,
    title: String,
    status: String,
    estimate: String,
    depends_on: Vec<String>,
    open_items: usize,
}

fn tasks(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let mut tasks = collect_tasks(&root.join(TASK_ROOT))?;
    tasks.sort_by(|a, b| a.gate.cmp(&b.gate).then_with(|| a.id.cmp(&b.id)));

    let problems = validate(&tasks);
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("error: {problem}");
        }
        return Err(format!("{} task problem(s)", problems.len()).into());
    }

    let rendered = render_index(&tasks);
    let out = root.join(TASK_INDEX);
    if check {
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        if current != rendered {
            eprintln!("{TASK_INDEX} is out of date; run `cargo xtask tasks`");
            std::process::exit(1);
        }
        println!("{TASK_INDEX} is up to date ({} tasks)", tasks.len());
        return Ok(());
    }
    std::fs::write(&out, rendered)?;
    println!("wrote {TASK_INDEX} ({} tasks)", tasks.len());
    Ok(())
}

fn collect_tasks(dir: &std::path::Path) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("_index.md") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            found.push(parse_task(&path, &text)?);
        }
    }
    Ok(found)
}

fn parse_task(path: &std::path::Path, text: &str) -> Result<Task, Box<dyn std::error::Error>> {
    let mut id = String::new();
    let mut gate = String::new();
    let mut title = String::new();
    let mut status = String::new();
    let mut estimate = String::new();
    let mut depends_on = Vec::new();

    let mut in_front_matter = false;
    let mut front_matter_closed = false;
    for line in text.lines() {
        if line.trim() == "---" {
            if in_front_matter {
                front_matter_closed = true;
                break;
            }
            in_front_matter = true;
            continue;
        }
        if !in_front_matter {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "id" => id = value,
            "gate" => gate = value,
            "title" => title = value,
            "status" => status = value,
            "estimate" => estimate = value,
            "depends_on" => {
                let inner = value.trim_start_matches('[').trim_end_matches(']');
                depends_on = inner
                    .split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    if !front_matter_closed {
        return Err(format!("{} has no front matter", path.display()).into());
    }

    let open_items = text.matches("- [ ]").count();

    Ok(Task {
        id,
        gate,
        title,
        status,
        estimate,
        depends_on,
        open_items,
    })
}

fn validate(tasks: &[Task]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = std::collections::BTreeMap::new();

    for task in tasks {
        if task.id.is_empty() {
            problems.push("a task is missing its `id`".to_string());
        }
        if let Some(previous) = seen.insert(task.id.clone(), task.status.clone()) {
            problems.push(format!("duplicate task id {} (status {previous})", task.id));
        }
        if !matches!(task.status.as_str(), "todo" | "doing" | "done" | "dropped") {
            problems.push(format!("{} has unknown status `{}`", task.id, task.status));
        }
        if task.status == "done" && task.open_items > 0 {
            problems.push(format!(
                "{} is done but has {} unchecked item(s)",
                task.id, task.open_items
            ));
        }
    }

    let ids: std::collections::BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    for task in tasks {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                problems.push(format!("{} depends on unknown task {}", task.id, dep));
            }
        }
    }

    // Cycle detection over depends_on. A cycle means following the dependency
    // edges from a task leads back to that same task; reaching a task twice by
    // two different routes (a diamond) is not a cycle, so this walks one path at
    // a time instead of remembering every node ever seen.
    let by_id: std::collections::BTreeMap<&str, &Task> =
        tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    fn returns_to(
        start: &str,
        node: &str,
        by_id: &std::collections::BTreeMap<&str, &Task>,
        path: &mut Vec<String>,
    ) -> bool {
        let Some(task) = by_id.get(node) else {
            return false;
        };
        for dep in &task.depends_on {
            if dep == start {
                return true;
            }
            if path.iter().any(|seen| seen == dep) {
                continue;
            }
            path.push(dep.clone());
            let found = returns_to(start, dep.as_str(), by_id, path);
            path.pop();
            if found {
                return true;
            }
        }
        false
    }

    for task in tasks {
        let mut path = vec![task.id.clone()];
        if returns_to(&task.id, &task.id, &by_id, &mut path) {
            problems.push(format!("dependency cycle involving {}", task.id));
        }
    }

    problems
}

fn render_index(tasks: &[Task]) -> String {
    let mut out = String::new();
    out.push_str("# 任务索引\n\n");
    out.push_str("> 由 `cargo xtask tasks` 生成，不要手改。`--check` 会校验 id 唯一、依赖存在、无环、done 任务无未勾选项。\n\n");

    let count = |status: &str| tasks.iter().filter(|t| t.status == status).count();
    let done = count("done");
    let total = tasks.len();
    out.push_str(&format!("进度：**{done}/{total}** 完成\n\n"));

    let next = next_task(tasks);
    match next {
        Some(task) => out.push_str(&format!(
            "**下一个可开工任务**：{} · {}\n\n",
            task.id, task.title
        )),
        None => out.push_str("**没有待开工任务**（或依赖未满足）\n\n"),
    }

    let mut gates: Vec<&str> = tasks.iter().map(|t| t.gate.as_str()).collect();
    gates.sort_unstable();
    gates.dedup();

    for gate in gates {
        out.push_str(&format!("## {gate}\n\n"));
        out.push_str("| id | 标题 | 状态 | 依赖 | 估时 |\n|---|---|---|---|---|\n");
        for task in tasks.iter().filter(|t| t.gate == gate) {
            let deps = if task.depends_on.is_empty() {
                "—".to_string()
            } else {
                task.depends_on.join(", ")
            };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                task.id, task.title, task.status, deps, task.estimate
            ));
        }
        out.push('\n');
    }

    out
}

fn next_task(tasks: &[Task]) -> Option<&Task> {
    let done: std::collections::BTreeSet<&str> = tasks
        .iter()
        .filter(|t| t.status == "done")
        .map(|t| t.id.as_str())
        .collect();
    tasks
        .iter()
        .filter(|t| t.status == "todo")
        .find(|t| t.depends_on.iter().all(|d| done.contains(d.as_str())))
}
