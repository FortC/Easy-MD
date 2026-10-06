//! md 文本解析：frontmatter / 标题 / 标签 / wikilink / 块 id。
//! 只读不写：解析结果供索引层使用，绝不改动原始文本。

use super::model::{BlockId, Heading, NoteLink};
use regex::Regex;
use std::sync::LazyLock;

static RE_WIKILINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(!?)\[\[([^\[\]]+?)\]\]").unwrap());
static RE_HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^ {0,3}(#{1,6})[ \t]+(\S.*?)[ \t]*$").unwrap());
// 标签：# 后跟字母/数字/下划线开头，允许多级 /；前面不能是字词字符（排除 [[a#b]]、url#frag、C#）
static RE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(^|[^\w&\\])#([\p{L}\p{N}_][\p{L}\p{N}_/-]*)").unwrap());
static RE_BLOCK_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\^([A-Za-z0-9-]+)\s*$").unwrap());
static RE_INLINE_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`+[^`\n]*`+").unwrap());
static RE_MD_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!?\[[^\]\[]*\]\([^)\n]*\)").unwrap());

/// 超过此大小（字节）的文件不解析正文，仅入索引骨架，防止 pathological 文件拖慢扫描
const MAX_PARSE_SIZE: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default, Clone)]
pub struct ParsedNote {
    pub aliases: Vec<String>,
    pub yaml_tags: Vec<String>,
    pub body_tags: Vec<String>,
    pub headings: Vec<Heading>,
    pub links: Vec<NoteLink>,
    pub block_ids: Vec<BlockId>,
}

/// 拆出 frontmatter：返回 (yaml 文本, 正文, 正文起始行号)
pub fn split_frontmatter(text: &str) -> (Option<&str>, &str, usize) {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let after = match t.strip_prefix("---\n") {
        Some(a) => a,
        None => {
            // 兼容 "---\r\n"
            match t.strip_prefix("---\r\n") {
                Some(a) => a,
                None => return (None, t, 0),
            }
        }
    };
    // 找结束分隔线：行首 "---"（后跟换行或结尾）
    let mut pos = 0usize;
    for line in after.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" || trimmed == "..." {
            let yaml = &after[..pos];
            let body = &after[pos + line.len()..];
            let body_line = yaml.matches('\n').count() + 2; // 开始线 + 结束线
            return (Some(yaml), body, body_line);
        }
        pos += line.len();
    }
    (None, t, 0) // 没有闭合的 frontmatter，当作普通正文
}

fn yaml_strings(v: &serde_yaml::Value) -> Vec<String> {
    match v {
        serde_yaml::Value::String(s) => vec![s.trim().to_string()],
        serde_yaml::Value::Number(n) => vec![n.to_string()],
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|x| match x {
                serde_yaml::Value::String(s) => Some(s.trim().to_string()),
                serde_yaml::Value::Number(n) => Some(n.to_string()),
                serde_yaml::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn line_of(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].matches('\n').count()
}

/// 解析一篇笔记的完整索引信息
pub fn parse_note(text: &str) -> ParsedNote {
    let mut out = ParsedNote::default();
    let (yaml, body, body_start_line) = split_frontmatter(text);

    // ---- frontmatter ----
    if let Some(yaml_str) = yaml {
        if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            if let Some(v) = v.get("aliases").or_else(|| v.get("alias")) {
                out.aliases = yaml_strings(v);
            }
            if let Some(v) = v.get("tags").or_else(|| v.get("tag")) {
                // yaml 里 tags 若是逗号分隔字符串，拆开
                out.yaml_tags = yaml_strings(v)
                    .into_iter()
                    .flat_map(|s| {
                        s.split(',')
                            .map(|x| x.trim().to_string())
                            .collect::<Vec<_>>()
                    })
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    // ---- wikilinks（在原始正文上提取，保证 raw 完整）----
    for cap in RE_WIKILINK.captures_iter(body) {
        let whole = cap.get(0).unwrap();
        let embed = &cap[1] == "!";
        let inner = cap[2].trim();
        let (target_part, alias) = match inner.split_once('|') {
            Some((t, a)) => (t.trim(), Some(a.trim().to_string())),
            None => (inner, None),
        };
        let (target, subpath) = match target_part.split_once('#') {
            Some((t, s)) if !s.is_empty() => (t.trim().to_string(), Some(s.to_string())),
            Some((t, _)) => (t.trim().to_string(), None),
            None => (target_part.to_string(), None),
        };
        out.links.push(NoteLink {
            raw: whole.as_str().to_string(),
            target,
            subpath,
            alias,
            embed,
            line: line_of(body, whole.start()) + body_start_line,
        });
    }

    // ---- 标题 ----
    for cap in RE_HEADING.captures_iter(body) {
        let whole = cap.get(0).unwrap();
        out.headings.push(Heading {
            level: cap[1].len() as u8,
            text: cap[2].trim().to_string(),
            line: line_of(body, whole.start()) + body_start_line,
        });
    }

    // ---- 块 id（行尾 ^id，逐行扫描更快）----
    for (i, line) in body.lines().enumerate() {
        if let Some(cap) = RE_BLOCK_ID.captures(line) {
            out.block_ids.push(BlockId {
                id: cap[1].to_string(),
                line: i + body_start_line,
            });
        }
    }

    // ---- 正文标签：先掩码代码/链接区域再扫描，避免误报 ----
    let masked = mask_code_and_links(body);
    for cap in RE_TAG.captures_iter(&masked) {
        let tag = cap[2].trim_end_matches('/').to_string();
        // 纯数字不算标签（如 #2024）
        if tag.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if !tag.is_empty() && !out.body_tags.contains(&tag) {
            out.body_tags.push(tag);
        }
    }

    out
}

/// 掩码会影响标签扫描的区域：围栏代码块、行内代码、wikilink、md 链接（保留换行结构）
fn mask_code_and_links(body: &str) -> String {
    // 1) 围栏代码块逐行掩码
    let mut fenced: Vec<String> = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        let is_fence = (trimmed.starts_with("```") || trimmed.starts_with("~~~"))
            && trimmed.len() >= 3;
        if is_fence && !in_fence {
            in_fence = true;
            fence_marker = trimmed[..3].to_string();
            fenced.push(String::new());
        } else if is_fence && in_fence && trimmed.starts_with(&fence_marker) {
            in_fence = false;
            fenced.push(String::new());
        } else if in_fence {
            fenced.push(String::new());
        } else {
            fenced.push(line.to_string());
        }
    }
    let mut s = fenced.join("\n");
    // 2) 行内代码 / wikilink / md 链接 → 用等长空格替换（保持长度，换行已保留）
    for re in [&*RE_INLINE_CODE, &*RE_WIKILINK, &*RE_MD_LINK] {
        s = re
            .replace_all(&s, |caps: &regex::Captures| {
                " ".repeat(caps.get(0).unwrap().as_str().len())
            })
            .to_string();
    }
    s
}

/// 判断是否应该跳过（体积过大）
pub fn too_large(size: u64) -> bool {
    size > MAX_PARSE_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_basic() {
        let text = "---\ntitle: hi\naliases: [a, b]\ntags:\n  - x/y\n  - z\n---\n# Head\nbody";
        let (yaml, body, line) = split_frontmatter(text);
        let yaml = yaml.unwrap();
        assert!(yaml.contains("aliases"));
        assert!(body.starts_with("# Head"));
        assert_eq!(line, 7);
        let p = parse_note(text);
        assert_eq!(p.aliases, vec!["a", "b"]);
        assert_eq!(p.yaml_tags, vec!["x/y", "z"]);
    }

    #[test]
    fn frontmatter_string_tags() {
        let text = "---\ntags: a, b/c\n---\nhello";
        let p = parse_note(text);
        assert_eq!(p.yaml_tags, vec!["a", "b/c"]);
    }

    #[test]
    fn wikilinks_variants() {
        let text = "# t\n[[Note A]] [[B|别名]] [[C#标题]] [[D#^abc1]] ![[E]] ![[img.png]] [[#同文件标题]]";
        let p = parse_note(text);
        assert_eq!(p.links.len(), 7);
        assert_eq!(p.links[0].target, "Note A");
        assert!(!p.links[0].embed);
        assert_eq!(p.links[1].alias.as_deref(), Some("别名"));
        assert_eq!(p.links[2].subpath.as_deref(), Some("标题"));
        assert_eq!(p.links[3].subpath.as_deref(), Some("^abc1"));
        assert!(p.links[4].embed);
        assert_eq!(p.links[5].embed, true);
        assert_eq!(p.links[6].target, "");
    }

    #[test]
    fn tags_from_body_only() {
        let text = "正文 #foo/bar 标签\n#顶级\n`代码 #fake`\n```\n块 #notag\n```\n[[a#head]] [x](u#f) C# 不算\n#2024 数字不算";
        let p = parse_note(text);
        assert!(p.body_tags.contains(&"foo/bar".to_string()));
        assert!(p.body_tags.contains(&"顶级".to_string()));
        assert!(!p.body_tags.contains(&"fake".to_string()));
        assert!(!p.body_tags.contains(&"notag".to_string()));
        assert!(!p.body_tags.contains(&"head".to_string()));
        assert!(!p.body_tags.contains(&"2024".to_string()));
    }

    #[test]
    fn headings_and_block_ids() {
        let text = "---\nk: v\n---\n# 一\n## 二\n内容 ^blk01\n";
        let p = parse_note(text);
        assert_eq!(p.headings.len(), 2);
        assert_eq!(p.headings[0].text, "一");
        assert_eq!(p.headings[0].line, 3);
        assert_eq!(p.block_ids[0].id, "blk01");
    }

    #[test]
    fn no_frontmatter() {
        let (yaml, _, _) = split_frontmatter("# just body");
        assert!(yaml.is_none());
    }
}
