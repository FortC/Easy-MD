//! 索引引擎：扫描 vault、增量更新、缓存读写、链接解析、反链计算。
//! 缓存仅是加速器：打开 vault 时会校验每个文件的 mtime，不一致即重新解析。

use super::model::{Backlink, NoteIndex, ResolveResult, TagCount};
use super::parser;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// 缓存结构版本号：模型变更时 +1，旧缓存自动失效重建
pub const CACHE_VERSION: u32 = 1;

#[derive(Default)]
pub struct IndexEngine {
    /// key: vault 内相对路径（'/' 分隔）
    pub notes: BTreeMap<String, NoteIndex>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    version: u32,
    notes: Vec<NoteIndex>,
}

/// 跳过隐藏目录 / node_modules
fn is_skipped_entry(name: &std::ffi::OsStr) -> bool {
    let s = name.to_string_lossy();
    s.starts_with('.') || s == "node_modules" || s == "$RECYCLE.BIN" || s == "System Volume Information"
}

fn rel_path(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .replace('\\', "/")
}

fn file_stem_of(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| rel.to_string())
}

impl IndexEngine {
    /// 全量扫描
    pub fn scan(root: &Path) -> Result<Self> {
        let mut engine = IndexEngine::default();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0 || !is_skipped_entry(e.file_name())
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue, // 无权限等错误：跳过该条目
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().map(|e| e.eq_ignore_ascii_case("md")).unwrap_or(false) {
                let rel = rel_path(root, entry.path());
                if let Some(note) = parse_file(root, &rel) {
                    engine.notes.insert(rel, note);
                }
            }
        }
        Ok(engine)
    }

    /// 打开 vault 时的增量校验：缓存命中且 mtime 一致则不重新解析
    pub fn refresh_against_disk(&mut self, root: &Path) {
        let mut disk_files = std::collections::HashSet::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_skipped_entry(e.file_name()))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().map(|e| e.eq_ignore_ascii_case("md")).unwrap_or(false) {
                let rel = rel_path(root, entry.path());
                disk_files.insert(rel.clone());
                let need_parse = match self.notes.get(&rel) {
                    Some(cached) => {
                        let meta = std::fs::metadata(entry.path());
                        match meta {
                            Ok(m) => m.len() != cached.size
                                || mtime_of(&m).unwrap_or(0) != cached.mtime,
                            Err(_) => true,
                        }
                    }
                    None => true,
                };
                if need_parse {
                    if let Some(note) = parse_file(root, &rel) {
                        self.notes.insert(rel, note);
                    }
                }
            }
        }
        // 清理磁盘上已不存在的缓存条目
        self.notes.retain(|k, _| disk_files.contains(k));
    }

    /// 单文件更新（监听器触发）
    pub fn update_file(&mut self, root: &Path, rel: &str) -> Option<NoteIndex> {
        let full = root.join(rel);
        if full.exists() {
            let note = parse_file(root, rel)?;
            self.notes.insert(rel.to_string(), note.clone());
            Some(note)
        } else {
            self.notes.remove(rel);
            None
        }
    }

    pub fn save_cache(&self, cache_path: &Path) {
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = CacheFile {
            version: CACHE_VERSION,
            notes: self.notes.values().cloned().collect(),
        };
        if let Ok(json) = serde_json::to_vec(&file) {
            let tmp = cache_path.with_extension("json.tmp");
            if std::fs::write(&tmp, &json).is_ok() {
                let _ = std::fs::rename(&tmp, cache_path);
            }
        }
    }

    pub fn load_cache(cache_path: &Path) -> Option<Self> {
        let bytes = std::fs::read(cache_path).ok()?;
        let file: CacheFile = serde_json::from_slice(&bytes).ok()?;
        if file.version != CACHE_VERSION {
            return None; // 旧缓存：弃用，走全量扫描
        }
        let mut notes = BTreeMap::new();
        for n in file.notes {
            notes.insert(n.path.clone(), n);
        }
        Some(IndexEngine { notes })
    }

    /// 缓存文件路径：<配置目录>/cache/index/<sha256(vault绝对路径)>.json
    pub fn cache_path(config_dir: &Path, vault_root: &Path) -> std::path::PathBuf {
        let key = Sha256::digest(vault_root.to_string_lossy().as_bytes());
        let hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();
        config_dir.join("cache").join("index").join(format!("{}.json", hex))
    }

    /// 解析链接目标：文件名（含路径）→ 别名，同名多个时 ambiguous
    pub fn resolve(&self, target: &str) -> ResolveResult {
        let t = target.trim();
        if t.is_empty() {
            return ResolveResult { path: None, ambiguous: false };
        }
        let t_norm = t.replace('\\', "/");
        let t_lower = t_norm.to_lowercase();

        // 1) 带路径的精确匹配：目标本身 或 目标.md
        if t_norm.contains('/') {
            let direct = self
                .notes
                .keys()
                .find(|k| k.to_lowercase() == t_lower || k.to_lowercase() == format!("{}.md", t_lower));
            if let Some(p) = direct {
                return ResolveResult { path: Some(p.clone()), ambiguous: false };
            }
        }

        // 2) 文件名匹配（目标带 .md 后缀时按去后缀比较，与 Obsidian 一致）
        let stem_lower = t_lower.strip_suffix(".md").unwrap_or(t_lower.as_str());
        let by_title: Vec<&String> = self
            .notes
            .keys()
            .filter(|k| file_stem_of(k).to_lowercase() == stem_lower)
            .collect();
        if by_title.len() == 1 {
            return ResolveResult { path: Some(by_title[0].clone()), ambiguous: false };
        }
        if by_title.len() > 1 {
            return ResolveResult { path: Some(by_title[0].clone()), ambiguous: true };
        }

        // 3) 别名匹配
        let by_alias: Vec<&String> = self
            .notes
            .iter()
            .filter(|(_, n)| n.aliases.iter().any(|a| a.to_lowercase() == t_lower))
            .map(|(k, _)| k)
            .collect();
        if by_alias.len() == 1 {
            return ResolveResult { path: Some(by_alias[0].clone()), ambiguous: false };
        }
        if by_alias.len() > 1 {
            return ResolveResult { path: Some(by_alias[0].clone()), ambiguous: true };
        }

        ResolveResult { path: None, ambiguous: false }
    }

    /// 反向链接：引用了 note_path 的所有其它笔记
    pub fn backlinks(&self, note_path: &str) -> Vec<Backlink> {
        let mut out = Vec::new();
        for (src, note) in &self.notes {
            if src == note_path {
                continue;
            }
            for link in &note.links {
                // 空目标 = 同文件引用，跳过
                if link.target.is_empty() {
                    continue;
                }
                if let Some(resolved) = self.resolve(&link.target).path {
                    if resolved == note_path {
                        out.push(Backlink {
                            source: src.clone(),
                            source_title: note.title.clone(),
                            link: link.clone(),
                        });
                    }
                }
            }
        }
        out
    }

    /// 标签计数（含多级标签的父级计数）
    pub fn tag_counts(&self) -> Vec<TagCount> {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for note in self.notes.values() {
            let mut seen = std::collections::HashSet::new();
            for tag in &note.tags {
                // 多级标签 a/b/c 同时计入 a、a/b、a/b/c
                let mut prefix = String::new();
                for part in tag.split('/') {
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    prefix.push_str(part);
                    if seen.insert(prefix.clone()) {
                        *counts.entry(prefix.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
        counts
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect()
    }

    pub fn all_notes(&self) -> Vec<NoteIndex> {
        self.notes.values().cloned().collect()
    }
}

pub fn mtime_of(meta: &std::fs::Metadata) -> Result<u64> {
    use std::time::UNIX_EPOCH;
    Ok(meta.modified().context("无修改时间")?.duration_since(UNIX_EPOCH)?.as_secs())
}

/// 读取并解析单个 md 文件
fn parse_file(root: &Path, rel: &str) -> Option<NoteIndex> {
    let full = root.join(rel);
    let bytes = std::fs::read(&full).ok()?;
    let meta = std::fs::metadata(&full).ok()?;
    let size = meta.len();
    let title = file_stem_of(rel);
    if parser::too_large(size) {
        return Some(NoteIndex {
            path: rel.to_string(),
            title,
            aliases: vec![],
            tags: vec![],
            headings: vec![],
            links: vec![],
            block_ids: vec![],
            mtime: mtime_of(&meta).unwrap_or(0),
            size,
        });
    }
    let text = String::from_utf8_lossy(&bytes);
    let parsed = parser::parse_note(&text);
    let mut tags = parsed.yaml_tags.clone();
    for t in &parsed.body_tags {
        if !tags.contains(t) {
            tags.push(t.clone());
        }
    }
    Some(NoteIndex {
        path: rel.to_string(),
        title,
        aliases: parsed.aliases,
        tags,
        headings: parsed.headings,
        links: parsed.links,
        block_ids: parsed.block_ids,
        mtime: mtime_of(&meta).unwrap_or(0),
        size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let full = root.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }

    #[test]
    fn scan_resolve_backlinks() {
        let dir = std::env::temp_dir().join(format!("emd_test_{}", uuid::Uuid::new_v4()));
        write(&dir, "a.md", "# A\n[[B]] [[别名C]]");
        write(&dir, "sub/b.md", "# B\ntext");
        write(&dir, "c.md", "---\naliases: [别名C]\n---\n[[B|另一个B]]");
        let engine = IndexEngine::scan(&dir).unwrap();
        assert_eq!(engine.notes.len(), 3);

        let r = engine.resolve("B");
        assert_eq!(r.path.as_deref(), Some("sub/b.md"));
        assert!(!r.ambiguous);

        let r = engine.resolve("别名C");
        assert_eq!(r.path.as_deref(), Some("c.md"));

        // 反链：b 被 a 和 c 引用
        let bl = engine.backlinks("sub/b.md");
        assert_eq!(bl.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tag_counts_multilevel() {
        let dir = std::env::temp_dir().join(format!("emd_test_{}", uuid::Uuid::new_v4()));
        write(&dir, "n.md", "#x/y #x\n#z");
        write(&dir, "m.md", "#x/y");
        let engine = IndexEngine::scan(&dir).unwrap();
        let counts = engine.tag_counts();
        let get = |t: &str| counts.iter().find(|c| c.tag == t).map(|c| c.count).unwrap_or(0);
        assert_eq!(get("x"), 2);
        assert_eq!(get("x/y"), 2);
        assert_eq!(get("z"), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 目标带 .md 后缀也能解析（Obsidian 行为：[[1.md]] 与 [[1]] 等价）
    #[test]
    fn resolve_accepts_md_suffix() {
        let dir = std::env::temp_dir().join(format!("emd_test_{}", uuid::Uuid::new_v4()));
        write(&dir, "1.md", "[[2]]");
        write(&dir, "sub/2.md", "内容");
        let engine = IndexEngine::scan(&dir).unwrap();
        assert_eq!(engine.resolve("1").path.as_deref(), Some("1.md"));
        assert_eq!(engine.resolve("1.md").path.as_deref(), Some("1.md"));
        assert_eq!(engine.resolve("sub/2").path.as_deref(), Some("sub/2.md"));
        assert_eq!(engine.resolve("sub/2.md").path.as_deref(), Some("sub/2.md"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 回归：中文/带空格/带点号文件名的反链（真实用户场景）
    #[test]
    fn backlinks_cjk_and_spaced_filenames() {
        let dir = std::env::temp_dir().join(format!("emd_test_{}", uuid::Uuid::new_v4()));
        write(&dir, "日记.md", "[[Test]] [[0. 新建笔记]] [[Test 2]] [[dail]] [[dddd]]");
        write(&dir, "Test.md", "内容");
        write(&dir, "Test 2.md", "内容");
        write(&dir, "dail.md", "内容");
        write(&dir, "dddd.md", "[[Test]]");
        write(&dir, "0. 新建笔记.md", "内容");
        let engine = IndexEngine::scan(&dir).unwrap();

        let bl = engine.backlinks("Test.md");
        assert_eq!(bl.len(), 2, "Test 应被 日记 和 dddd 引用: {:?}", bl.iter().map(|b| &b.source).collect::<Vec<_>>());
        assert_eq!(engine.backlinks("dddd.md").len(), 1);
        assert_eq!(engine.backlinks("Test 2.md").len(), 1);
        assert_eq!(engine.backlinks("dail.md").len(), 1);
        assert_eq!(engine.backlinks("0. 新建笔记.md").len(), 1);
        assert_eq!(engine.backlinks("日记.md").len(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }
}
