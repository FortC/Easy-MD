//! 索引数据模型：仅是磁盘 md 文件的内存缓存，可随时删除重建。

use serde::{Deserialize, Serialize};

/// 一篇笔记的索引信息（key 为 vault 内相对路径，`/` 分隔）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoteIndex {
    /// 相对路径，如 "notes/foo.md"
    pub path: String,
    /// 文件名（不含扩展名），链接解析的一级依据
    pub title: String,
    /// frontmatter aliases 字段
    pub aliases: Vec<String>,
    /// 全部标签（frontmatter tags + 正文 #tag），多级标签保持 "a/b" 形式
    pub tags: Vec<String>,
    /// 标题结构（大纲/标题跳转用）
    pub headings: Vec<Heading>,
    /// 笔记内所有 [[ ]] 与 ![[ ]] 链接
    pub links: Vec<NoteLink>,
    /// 正文里手工定义的块 id（行尾 ^id）
    pub block_ids: Vec<BlockId>,
    pub mtime: u64,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// 行号（0 基，按整个文件计，含 frontmatter 行）
    pub line: usize,
}

/// 内部链接
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoteLink {
    /// 原始文本，如 "![[foo#^abc1|别名]]"
    pub raw: String,
    /// 目标笔记名（可含路径、可为空=当前笔记）
    pub target: String,
    /// 子路径：Some("标题") 或 Some("^块id")
    pub subpath: Option<String>,
    /// 显示别名
    pub alias: Option<String>,
    /// 是否嵌入 ![[ ]]
    pub embed: bool,
    pub line: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockId {
    pub id: String,
    pub line: usize,
}

/// 反向链接条目：谁引用了我
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Backlink {
    pub source: String,
    pub source_title: String,
    pub link: NoteLink,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

/// 链接解析结果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResolveResult {
    pub path: Option<String>,
    /// 同名/同别名笔记不止一个时为 true，由前端提示消歧
    pub ambiguous: bool,
}
