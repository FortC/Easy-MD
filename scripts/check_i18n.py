# -*- coding: utf-8 -*-
# 扫描 .vue 里真实渲染的硬编码中文（排除 HTML 注释 / JS 注释 / AI 提示词长串）
import re
import sys
import io
import glob

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

for f in glob.glob(r"G:\pjs\easymd\src\**\*.vue", recursive=True):
    text = open(f, encoding="utf-8").read()
    # 去掉 HTML 注释
    text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
    # 去掉 JS 行注释
    lines = []
    for i, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("*") or stripped.startswith("/*"):
            continue
        lines.append((i, line))
    for i, line in lines:
        for cm in re.finditer(r"[\u4e00-\u9fff][\u4e00-\u9fff\u3000-\u303f\uff00-\uffef\w:/\\{}.,！？…（）&;'\-\[\]]*", line):
            s = cm.group(0).strip()
            # 过滤：块级字符串（AI 提示词，>10 连续汉字的长句归为提示词，人工复核）
            print(f"  {f.split(chr(92))[-1]}:{i}: {s[:70]}")
            break
