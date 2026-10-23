use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PAGE_WIDTH_MM: f64 = 210.0;
pub const PAGE_HEIGHT_MM: f64 = 297.0;
pub const PAGE_COUNT: usize = 9;
const MAX_ELEMENTS_PER_PAGE: usize = 80;
const MAX_TEXT_CHARS: usize = 5000;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ReportLayout {
    pub version: u32,
    pub pages: Vec<PageLayout>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PageLayout {
    pub page: usize,
    pub blocks: BTreeMap<String, BlockLayout>,
    pub elements: Vec<LayoutElement>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BlockLayout {
    pub dx: f64,
    pub dy: f64,
    pub hidden: bool,
    pub width: f64,
    pub height: f64,
    pub background: String,
    pub border_color: String,
    pub border_width: f64,
    pub radius: f64,
    pub texts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LayoutElement {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    pub z: i32,
    pub hidden: bool,
    pub src: String,
    pub alt: String,
    pub object_fit: String,
    pub fill: String,
    pub border_color: String,
    pub border_width: f64,
    pub radius: f64,
    pub text: String,
    pub font_size: f64,
    pub font_family: String,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub align: String,
    pub background: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutView {
    pub layout: ReportLayout,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveLayoutRequest {
    pub import_id: String,
    pub layout: ReportLayout,
}

#[derive(Clone, Debug, Serialize)]
pub struct UploadedImage {
    pub id: String,
    pub url: String,
}

impl Default for LayoutElement {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: String::new(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            rotation: 0.0,
            opacity: 1.0,
            z: 0,
            hidden: false,
            src: String::new(),
            alt: String::new(),
            object_fit: String::new(),
            fill: String::new(),
            border_color: String::new(),
            border_width: 0.0,
            radius: 0.0,
            text: String::new(),
            font_size: 0.0,
            font_family: String::new(),
            color: String::new(),
            bold: false,
            italic: false,
            align: String::new(),
            background: String::new(),
        }
    }
}

impl ReportLayout {
    pub fn sanitize(self) -> Self {
        let mut slots: Vec<Option<PageLayout>> = (0..PAGE_COUNT).map(|_| None).collect();
        let mut used_element_ids = HashSet::new();

        for page in self.pages {
            if page.page == 0 || page.page > PAGE_COUNT {
                continue;
            }
            let index = page.page - 1;
            if slots[index].is_some() {
                continue;
            }
            slots[index] = Some(page.sanitize(&mut used_element_ids));
        }

        let pages = slots
            .into_iter()
            .enumerate()
            .map(|(index, page)| {
                page.unwrap_or_else(|| PageLayout {
                    page: index + 1,
                    ..PageLayout::default()
                })
            })
            .collect();

        Self { version: 1, pages }
    }
}

impl PageLayout {
    fn sanitize(self, used_element_ids: &mut HashSet<String>) -> Self {
        let page = self.page;
        let blocks = self
            .blocks
            .into_iter()
            .filter_map(|(id, block)| {
                if !valid_id(&id) {
                    return None;
                }
                let dx = clamp_num(block.dx, -1000.0, 1000.0, 0.0);
                let dy = clamp_num(block.dy, -1000.0, 1000.0, 0.0);
                let width = clamp_num(block.width, 0.0, PAGE_WIDTH_MM, 0.0);
                let height = clamp_num(block.height, 0.0, PAGE_HEIGHT_MM, 0.0);
                let background = optional_color(&block.background);
                let border_color = optional_color(&block.border_color);
                let border_width = clamp_num(block.border_width, 0.0, 20.0, 0.0);
                let radius = clamp_num(block.radius, 0.0, 100.0, 0.0);
                let texts = block
                    .texts
                    .into_iter()
                    .filter(|(text_id, _)| valid_id(text_id))
                    .take(50)
                    .map(|(text_id, value)| (text_id, normalize_text(&value)))
                    .collect::<BTreeMap<_, _>>();

                let is_empty = !block.hidden
                    && dx.abs() < 0.005
                    && dy.abs() < 0.005
                    && width == 0.0
                    && height == 0.0
                    && background.is_empty()
                    && border_width == 0.0
                    && radius == 0.0
                    && texts.is_empty();
                if is_empty {
                    return None;
                }

                Some((
                    id,
                    BlockLayout {
                        dx,
                        dy,
                        hidden: block.hidden,
                        width,
                        height,
                        background,
                        border_color,
                        border_width,
                        radius,
                        texts,
                    },
                ))
            })
            .collect();

        let elements = self
            .elements
            .into_iter()
            .take(MAX_ELEMENTS_PER_PAGE)
            .filter_map(|element| element.sanitize(used_element_ids))
            .collect();

        Self {
            page,
            blocks,
            elements,
        }
    }
}

impl LayoutElement {
    fn sanitize(mut self, used_ids: &mut HashSet<String>) -> Option<Self> {
        self.kind = self.kind.trim().to_ascii_lowercase();
        if !matches!(self.kind.as_str(), "image" | "rect" | "text") {
            return None;
        }

        if !valid_id(&self.id) || !used_ids.insert(self.id.clone()) {
            self.id = Uuid::new_v4().to_string();
            used_ids.insert(self.id.clone());
        }

        let (default_width, default_height) = match self.kind.as_str() {
            "image" => (60.0, 45.0),
            "rect" => (40.0, 20.0),
            _ => (60.0, 15.0),
        };
        self.width = clamp_num(self.width, 5.0, PAGE_WIDTH_MM, default_width);
        self.height = clamp_num(self.height, 3.0, PAGE_HEIGHT_MM, default_height);
        self.x = clamp_num(self.x, 0.0, PAGE_WIDTH_MM - self.width, 10.0);
        self.y = clamp_num(self.y, 0.0, PAGE_HEIGHT_MM - self.height, 10.0);
        self.rotation = clamp_num(self.rotation, -360.0, 360.0, 0.0);
        self.opacity = clamp_num(self.opacity, 0.0, 1.0, 1.0);
        self.z = self.z.clamp(-100, 1000);
        self.alt = truncate(&self.alt, 200);
        self.text = normalize_text(&self.text);

        match self.kind.as_str() {
            "image" => {
                self.src = safe_image_src(&self.src)?;
                self.object_fit = match self.object_fit.trim().to_ascii_lowercase().as_str() {
                    "cover" => "cover".to_string(),
                    "fill" => "fill".to_string(),
                    _ => "contain".to_string(),
                };
            }
            "rect" => {
                self.src.clear();
                self.alt.clear();
                self.fill = safe_color(&self.fill, "#ffffff");
                self.border_color = safe_color(&self.border_color, "#143d30");
                self.border_width = clamp_num(self.border_width, 0.0, 20.0, 0.5);
                self.radius = clamp_num(self.radius, 0.0, 100.0, 1.0);
            }
            "text" => {
                self.src.clear();
                self.alt.clear();
                self.font_size = clamp_num(self.font_size, 6.0, 72.0, 11.0);
                self.font_family = if self.font_family.trim().eq_ignore_ascii_case("serif") {
                    "serif".to_string()
                } else {
                    "sans".to_string()
                };
                self.color = safe_color(&self.color, "#143d30");
                self.background = safe_color(&self.background, "transparent");
                self.align = match self.align.trim().to_ascii_lowercase().as_str() {
                    "center" => "center".to_string(),
                    "right" => "right".to_string(),
                    _ => "left".to_string(),
                };
            }
            _ => unreachable!(),
        }

        Some(self)
    }
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub fn default_opacity() -> f64 {
    1.0
}

pub fn image_id_from_src(src: &str) -> Option<&str> {
    let id = src.strip_prefix("upload:")?;
    valid_id(id).then_some(id)
}

fn safe_image_src(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(id) = value.strip_prefix("upload:") {
        return valid_id(id).then(|| format!("upload:{id}"));
    }
    let lower = value.to_ascii_lowercase();
    let is_http = lower.starts_with("http://") || lower.starts_with("https://");
    let safe = value.len() <= 2048
        && is_http
        && !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_ascii_whitespace());
    safe.then(|| value.to_string())
}

fn optional_color(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    safe_color(value, "")
}

fn safe_color(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.eq_ignore_ascii_case("transparent") || value.eq_ignore_ascii_case("none") {
        return value.to_ascii_lowercase();
    }
    if let Some(hex) = value.strip_prefix('#') {
        if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return value.to_string();
        }
    }
    fallback.to_string()
}

fn clamp_num(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    round3(value.clamp(min, max))
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn normalize_text(value: &str) -> String {
    truncate(
        &value.replace("\r\n", "\n").replace('\r', "\n"),
        MAX_TEXT_CHARS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_bounds_ids_and_colors() {
        let layout = ReportLayout {
            version: 0,
            pages: vec![PageLayout {
                page: 2,
                blocks: BTreeMap::from([
                    (
                        "page-2-head".to_string(),
                        BlockLayout {
                            dx: 5000.0,
                            dy: -5000.0,
                            hidden: false,
                            ..Default::default()
                        },
                    ),
                    (
                        "bad id".to_string(),
                        BlockLayout {
                            dx: 10.0,
                            dy: 0.0,
                            hidden: false,
                            ..Default::default()
                        },
                    ),
                ]),
                elements: vec![
                    LayoutElement {
                        id: "bad id".to_string(),
                        kind: "image".to_string(),
                        x: -50.0,
                        y: 500.0,
                        width: 500.0,
                        height: 1.0,
                        src: "https://example.com/a.png".to_string(),
                        ..Default::default()
                    },
                    LayoutElement {
                        id: "rect-1".to_string(),
                        kind: "rect".to_string(),
                        fill: "red;display:none".to_string(),
                        ..Default::default()
                    },
                    LayoutElement {
                        id: "video-1".to_string(),
                        kind: "video".to_string(),
                        ..Default::default()
                    },
                ],
            }],
        };

        let sanitized = layout.sanitize();
        assert_eq!(sanitized.pages.len(), PAGE_COUNT);
        let page = &sanitized.pages[1];
        assert_eq!(page.blocks.get("page-2-head").unwrap().dx, 1000.0);
        assert_eq!(page.blocks.get("page-2-head").unwrap().dy, -1000.0);
        assert!(!page.blocks.contains_key("bad id"));
        assert_eq!(page.elements.len(), 2);

        let image = page
            .elements
            .iter()
            .find(|item| item.kind == "image")
            .unwrap();
        assert_ne!(image.id, "bad id");
        assert_eq!(image.x, 0.0);
        assert_eq!(image.y, PAGE_HEIGHT_MM - image.height);
        assert_eq!(image.width, PAGE_WIDTH_MM);

        let rect = page
            .elements
            .iter()
            .find(|item| item.kind == "rect")
            .unwrap();
        assert_eq!(rect.fill, "#ffffff");
        assert!(!page.elements.iter().any(|item| item.kind == "video"));
    }
}
