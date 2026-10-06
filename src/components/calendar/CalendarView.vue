<template>
  <div class="calendar-view">
    <!-- 头部：年月导航 -->
    <div class="cal-header">
      <button class="cal-nav-btn" :title="t('cal.prevYear')" @click="shiftMonth(-12)">
        <Icon name="chevrons-left" :size="14" />
      </button>
      <button class="cal-nav-btn" :title="t('cal.prevMonth')" @click="shiftMonth(-1)">
        <Icon name="chevron-left" :size="14" />
      </button>
      <div class="cal-title">
        <span class="cal-y">{{ viewYear }}</span>
        <span class="cal-ym"> / </span>
        <span class="cal-m">{{ viewMonth }}</span>
      </div>
      <button class="cal-nav-btn" :title="t('cal.nextMonth')" @click="shiftMonth(1)">
        <Icon name="chevron-right" :size="14" />
      </button>
      <button class="cal-nav-btn" :title="t('cal.nextYear')" @click="shiftMonth(12)">
        <Icon name="chevrons-right" :size="14" />
      </button>
      <button class="emd-btn cal-today" @click="goToday">{{ t("cal.today") }}</button>
      <span class="cal-spacer" />
      <button class="emd-btn" @click="closeCalendar">
        <Icon name="x" :size="12" /> {{ t("c.close") }}
      </button>
    </div>

    <div class="cal-body">
      <!-- 月历格子 -->
      <div class="cal-grid-area">
        <div class="cal-weekdays">
          <span v-for="d in weekdays" :key="d" class="cal-wd">{{ d }}</span>
        </div>
        <div class="cal-grid">
          <button
            v-for="(cell, i) in cells"
            :key="i"
            class="cal-cell"
            :class="{
              'is-other-month': !cell.current,
              'is-today': cell.isToday,
              'is-selected': cell.date === selectedDate,
            }"
            @click="selectDate(cell.date)"
          >
            <span class="cal-day-num">{{ cell.day }}</span>
            <div v-if="cell.total > 0" class="cal-stats">
              <span v-if="cell.notes > 0" class="cal-stat">{{ tf("cal.statNote", { n: cell.notes }) }}</span>
              <span v-if="cell.jots > 0" class="cal-stat">{{ tf("cal.statJot", { n: cell.jots }) }}</span>
              <span v-if="cell.canvases > 0" class="cal-stat">{{ tf("cal.statCanvas", { n: cell.canvases }) }}</span>
              <span v-if="cell.graphs > 0" class="cal-stat">{{ tf("cal.statGraph", { n: cell.graphs }) }}</span>
              <span v-if="cell.images > 0" class="cal-stat">{{ tf("cal.statImage", { n: cell.images }) }}</span>
            </div>
          </button>
        </div>
      </div>

      <!-- 右侧：分组文件列表 -->
      <div class="cal-side">
        <div class="emd-panel-header">
          <span>{{ selectedDate || t("cal.pickDate") }}</span>
        </div>

        <!-- 分类文件列表 -->
        <div class="cal-file-list">
          <template v-for="group in groupedFiles" :key="group.type">
            <div v-if="group.files.length > 0" class="cal-group">
              <div class="cal-group-header">{{ group.label }} ({{ group.files.length }})</div>
              <div
                v-for="f in group.files"
                :key="f.path"
                class="emd-tree-item cal-file-item"
                @click="openFile(f)"
              >
                <Icon :name="group.icon" :size="13" class="cal-file-icon" />
                <span class="cal-file-name">{{ f.path.split("/").pop() }}</span>
              </div>
            </div>
          </template>
          <div v-if="!selectedDate" class="cal-side-empty">{{ t("cal.pickDateHint") }}</div>
          <div v-else-if="dayFiles.length === 0" class="cal-side-empty">{{ t("cal.noFiles") }}</div>
        </div>
      </div>
    </div>

    <!-- 底部：AI 日志生成器（通栏，重点功能） -->
    <div v-if="selectedDate" class="cal-ai-bar" :class="{ 'is-collapsed': !aiBarOpen }">
      <div class="cal-ai-head">
        <Icon name="play" :size="13" class="cal-ai-head-icon" />
        <span class="cal-ai-head-title">{{ t("cal.aiLog") }}</span>
        <span class="cal-ai-head-sub">{{ selectedDate }}</span>
        <button
          ref="tplSelBtn"
          class="cal-ai-tpl-btn"
          :title="t('cal.logTpl')"
          @click="tplSelOpen = !tplSelOpen"
        >
          <Icon name="file-text" :size="11" />
          {{ tf("cal.logTplWith", { name: logTplName || t("cal.logTplDefault") }) }}
          <Icon name="chevron-down" :size="11" />
        </button>
        <button class="cal-ai-fold" :title="aiBarOpen ? t('cal.fold') : t('cal.unfold')" @click="aiBarOpen = !aiBarOpen">
          <Icon :name="aiBarOpen ? 'chevron-down' : 'chevron-right'" :size="14" />
        </button>
      </div>

      <template v-if="aiBarOpen">
        <div v-if="dayFiles.length > 0" class="cal-ai-body">
          <!-- 左：对话 + 输入 -->
          <div class="cal-ai-left">
            <div class="cal-ai-msgs">
              <div v-if="aiMessages.length === 0" class="cal-ai-placeholder">
                {{ t("cal.aiHint") }}
              </div>
              <div
                v-for="(msg, i) in aiMessages"
                :key="i"
                class="cal-ai-msg"
                :class="msg.role"
              >
                {{ msg.text }}
              </div>
            </div>
            <div v-if="!aiRunning" class="cal-ai-input-row">
              <button class="emd-btn emd-btn-accent cal-ai-gen" @click="generateLog">
                <Icon name="play" :size="12" /> {{ t("cal.generate") }}
              </button>
              <input
                v-model="aiInput"
                type="text"
                class="cal-ai-input"
                :placeholder="t('cal.aiInputPh')"
                @keydown.enter="sendAiMessage"
              />
            </div>
            <div v-else class="cal-ai-loading">
              <Icon name="refresh-cw" :size="13" /> {{ t("ai.thinking") }}
            </div>
          </div>

          <!-- 右：生成结果（可编辑） -->
          <div class="cal-ai-right">
            <textarea
              v-if="generatedLog"
              v-model="generatedLog"
              class="cal-ai-result-input"
            />
            <div v-else class="cal-ai-placeholder cal-ai-result-empty">
              {{ t("cal.resultEmpty") }}
            </div>
            <div v-if="generatedLog" class="cal-ai-actions">
              <button class="emd-btn emd-btn-accent" @click="saveLog">
                <Icon name="check" :size="12" /> {{ t("cal.saveLog") }}
              </button>
            </div>
          </div>
        </div>
        <div v-else class="cal-ai-placeholder cal-ai-nofiles">{{ t("cal.noFiles") }}</div>
      </template>
    </div>

    <!-- 日志模板选择下拉 -->
    <DropdownMenu :open="tplSelOpen" :anchor="tplSelBtn" :items="tplSelItems" @select="onTplSel" @close="tplSelOpen = false" />

    <!-- 日志模板管理弹窗（长文编辑） -->
    <transition name="emd-fade">
      <div v-if="tplModalOpen" class="emd-modal-bg" @click.self="tplModalOpen = false">
        <div class="emd-modal cal-tpl-modal">
          <div class="cal-tpl-head">
            <span class="cal-tpl-title">{{ t("cal.logTpl") }}</span>
            <span class="cal-tpl-dir">{{ tf("cal.tplDirHint", { dir: logTplDir }) }}</span>
            <button class="emd-btn" @click="tplModalOpen = false">
              <Icon name="x" :size="12" /> {{ t("c.close") }}
            </button>
          </div>
          <div class="cal-tpl-body">
            <!-- 左：模板列表 -->
            <div class="cal-tpl-side">
              <div class="cal-tpl-side-btns">
                <button class="emd-btn emd-btn-accent" :title="t('cal.tplNew')" @click="createLogTpl">
                  <Icon name="plus" :size="13" />
                </button>
                <button class="emd-btn" :title="t('tp2.refresh')" @click="refreshLogTemplates">
                  <Icon name="refresh-cw" :size="13" />
                </button>
              </div>
              <div
                class="emd-tree-item cal-tpl-item"
                :class="{ 'is-active': tplEditing === null }"
                @click="loadTpl(null)"
              >
                <Icon name="file-text" :size="13" />
                <span class="cal-tpl-name">{{ t("cal.logTplDefault") }}</span>
              </div>
              <div
                v-for="tp in logTemplates"
                :key="tp.path"
                class="emd-tree-item cal-tpl-item"
                :class="{ 'is-active': tplEditing?.path === tp.path }"
                @click="loadTpl(tp)"
              >
                <Icon name="file-text" :size="13" />
                <span class="cal-tpl-name" :title="tp.path">{{ tp.name }}</span>
              </div>
              <div v-if="logTemplates.length === 0" class="cal-tpl-side-empty">
                {{ t("cal.tplListEmpty") }}
              </div>
            </div>
            <!-- 右：长文编辑器 -->
            <div class="cal-tpl-editor">
              <div class="cal-tpl-edit-head">
                <span class="cal-tpl-edit-name">{{ tplEditing ? tplEditing.name : t("cal.logTplDefault") }}</span>
                <template v-if="tplEditing">
                  <button class="emd-btn" @click="renameLogTpl">
                    <Icon name="pencil" :size="12" /> {{ t("cal.tplRename") }}
                  </button>
                  <button class="emd-btn cal-tpl-danger" @click="deleteLogTpl">
                    <Icon name="trash-2" :size="12" /> {{ t("c.delete") }}
                  </button>
                </template>
                <button v-else class="emd-btn" @click="tplContent = DEFAULT_LOG_TEMPLATE">
                  {{ t("cal.tplReset") }}
                </button>
              </div>
              <textarea
                v-model="tplContent"
                class="cal-tpl-textarea"
                :placeholder="t('cal.tplLongPh')"
              />
              <div class="cal-tpl-edit-actions">
                <span class="cal-tpl-vars">{{ t("cal.tplVars") }}</span>
                <button class="emd-btn emd-btn-accent" @click="saveTplEdit">{{ t("c.save") }}</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import DropdownMenu, { type DropItem } from "../common/DropdownMenu.vue";
import { api } from "../../ipc/tauri";
import { useUiStore } from "../../stores/ui";
import { useEditorStore } from "../../stores/editor";
import { useSettingsStore } from "../../stores/settings";
import { useVaultStore } from "../../stores/vault";
import {
  listLogTemplates,
  logTemplatesDir,
  DEFAULT_LOG_TEMPLATE,
  LOG_TEMPLATE_STARTER,
  type LogTemplateFile,
} from "../../lib/logTemplate";
import { t, tf } from "../../i18n";

const ui = useUiStore();
const editor = useEditorStore();
const settings = useSettingsStore();
const vault = useVaultStore();

const now = new Date();
const viewYear = ref(now.getFullYear());
const viewMonth = ref(now.getMonth() + 1);
const selectedDate = ref("");
const allFiles = ref<{ path: string; mtime: number; kind: string }[]>([]);

// AI 日志
const aiInput = ref("");
const aiRunning = ref(false);
const aiMessages = ref<{ role: "user" | "assistant"; text: string }[]>([]);
const generatedLog = ref("");
const aiBarOpen = ref(true);

// 日志模板（文件化规则，参考笔记模板）
const logTemplates = ref<LogTemplateFile[]>([]);
/** 选中的模板名，空 = 内置默认 */
const logTplName = ref("");
const tplSelOpen = ref(false);
const tplSelBtn = ref<HTMLElement>();
const tplModalOpen = ref(false);
const tplEditing = ref<LogTemplateFile | null>(null);
const tplContent = ref("");
const logTplDir = computed(() => logTemplatesDir());

const tplSelItems = computed<DropItem[]>(() => [
  { key: "d:", label: t("cal.logTplDefault"), icon: "file-text" },
  ...logTemplates.value.map((tp) => ({ key: `f:${tp.name}`, label: tp.name, icon: "file-text" })),
  { key: "sep", label: "", separator: true },
  { key: "manage", label: t("cal.tplManage"), icon: "pencil" },
]);

async function refreshLogTemplates() {
  logTemplates.value = await listLogTemplates();
}

async function onTplSel(key: string) {
  tplSelOpen.value = false;
  if (key === "manage") {
    tplModalOpen.value = true;
    await refreshLogTemplates();
    loadTpl(logTplName.value ? logTemplates.value.find((x) => x.name === logTplName.value) ?? null : null);
    return;
  }
  const name = key.startsWith("f:") ? key.slice(2) : "";
  logTplName.value = name;
  settings.data.log_template_name = name;
  await settings.persist();
}

/** 当前生效的生成规则（文件模板全文，或内置默认） */
async function currentRules(): Promise<string> {
  if (logTplName.value) {
    const f = logTemplates.value.find((x) => x.name === logTplName.value);
    if (f) {
      try {
        return await api.readTextFile(f.path);
      } catch {
        /* 文件丢失则回落默认 */
      }
    }
  }
  return settings.data.log_template || DEFAULT_LOG_TEMPLATE;
}

async function loadTpl(tp: LogTemplateFile | null) {
  tplEditing.value = tp;
  if (tp) {
    try {
      tplContent.value = await api.readTextFile(tp.path);
    } catch {
      tplContent.value = "";
    }
  } else {
    tplContent.value = settings.data.log_template || DEFAULT_LOG_TEMPLATE;
  }
}

async function saveTplEdit() {
  if (tplEditing.value) {
    try {
      await api.writeTextFile(tplEditing.value.path, tplContent.value);
    } catch (e) {
      alert(String(e));
    }
  } else {
    settings.data.log_template = tplContent.value;
    await settings.persist();
  }
}

async function createLogTpl() {
  const name = prompt(t("cal.tplNamePh"));
  if (!name || !name.trim()) return;
  const dir = logTemplatesDir();
  let rel = `${dir}/${name.trim()}.md`;
  try {
    if (await api.pathExists(rel)) {
      alert(t("cal.tplExists"));
      return;
    }
    await api.writeTextFile(rel, LOG_TEMPLATE_STARTER);
    await vault.refreshParents(rel);
    await refreshLogTemplates();
    await loadTpl(logTemplates.value.find((x) => x.path === rel) ?? null);
  } catch (e) {
    alert(String(e));
  }
}

async function renameLogTpl() {
  if (!tplEditing.value) return;
  const name = prompt(t("cal.tplNamePh"), tplEditing.value.name);
  if (!name || !name.trim() || name.trim() === tplEditing.value.name) return;
  try {
    const newPath = await api.renamePath(tplEditing.value.path, name.trim());
    await vault.refreshParents(newPath);
    await refreshLogTemplates();
    await loadTpl(logTemplates.value.find((x) => x.path === newPath) ?? null);
    if (logTplName.value) logTplName.value = logTemplates.value.find((x) => x.path === newPath)?.name ?? logTplName.value;
  } catch (e) {
    alert(String(e));
  }
}

async function deleteLogTpl() {
  if (!tplEditing.value) return;
  if (!confirm(tf("cal.tplDelConfirm", { name: tplEditing.value.name }))) return;
  try {
    const path = tplEditing.value.path;
    const name = tplEditing.value.name;
    await api.deletePath(path);
    await vault.refreshParents(path);
    await refreshLogTemplates();
    if (logTplName.value === name) {
      logTplName.value = "";
      settings.data.log_template_name = "";
      await settings.persist();
    }
    await loadTpl(null);
  } catch (e) {
    alert(String(e));
  }
}

const weekdays = computed(() => [1, 2, 3, 4, 5, 6, 7].map((d) => t(`cal.wd${d}`)));

function mtimeToDateStr(unixSec: number): string {
  const d = new Date(unixSec * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

// ---- 类型判断 ----
function isJot(path: string): boolean {
  return (
    path.startsWith("jots/") ||
    path.startsWith("daily/") ||
    path.startsWith((settings.data.daily_dir || "jots") + "/") ||
    path.includes("小计-")
  );
}

interface DayBreakdown {
  notes: number;
  jots: number;
  canvases: number;
  graphs: number;
  images: number;
  total: number;
}

const dateBreakdownMap = computed(() => {
  const map: Record<string, DayBreakdown> = {};
  for (const f of allFiles.value) {
    const d = mtimeToDateStr(f.mtime);
    if (!d) continue;
    if (!map[d]) map[d] = { notes: 0, jots: 0, canvases: 0, graphs: 0, images: 0, total: 0 };
    const b = map[d];
    b.total++;
    if (f.kind === "canvas") b.canvases++;
    else if (f.kind === "graph") b.graphs++;
    else if (f.kind === "image") b.images++;
    else if (isJot(f.path)) b.jots++;
    else b.notes++;
  }
  return map;
});

// ---- 月历 42 格 ----
const cells = computed(() => {
  const y = viewYear.value;
  const m = viewMonth.value;
  const first = new Date(y, m - 1, 1);
  const startOffset = (first.getDay() + 6) % 7;
  const start = new Date(y, m - 1, 1 - startOffset);
  const todayStr = mtimeToDateStr(Math.floor(Date.now() / 1000));

  const out: (DayBreakdown & { day: number; date: string; current: boolean; isToday: boolean })[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    const dateStr = mtimeToDateStr(Math.floor(d.getTime() / 1000));
    const b = dateBreakdownMap.value[dateStr] || { notes: 0, jots: 0, canvases: 0, graphs: 0, images: 0, total: 0 };
    out.push({ day: d.getDate(), date: dateStr, current: d.getMonth() + 1 === m, isToday: dateStr === todayStr, ...b });
  }
  return out;
});

// ---- 选中日期的文件 ----
const dayFiles = computed(() => {
  if (!selectedDate.value) return [];
  return allFiles.value.filter((f) => mtimeToDateStr(f.mtime) === selectedDate.value);
});

// ---- 分组展示 ----
const groupedFiles = computed(() => {
  const files = dayFiles.value;
  return [
    {
      type: "notes",
      label: t("cal.noteN"),
      icon: "file-text",
      files: files.filter((f) => f.kind === "md" && !isJot(f.path)),
    },
    {
      type: "jots",
      label: t("cal.jotN"),
      icon: "pencil",
      files: files.filter((f) => f.kind === "md" && isJot(f.path)),
    },
    {
      type: "canvases",
      label: t("cal.canvasN"),
      icon: "layout-grid",
      files: files.filter((f) => f.kind === "canvas"),
    },
    {
      type: "graphs",
      label: t("cal.graphN"),
      icon: "share-2",
      files: files.filter((f) => f.kind === "graph"),
    },
    {
      type: "images",
      label: t("cal.imageN"),
      icon: "image",
      files: files.filter((f) => f.kind === "image"),
    },
  ].filter((g) => g.files.length > 0);
});

// ---- AI 日志生成 ----
async function generateLog() {
  if (!selectedDate.value || dayFiles.value.length === 0) return;
  aiRunning.value = true;
  aiMessages.value = [];
  generatedLog.value = "";
  aiMessages.value.push({ role: "user", text: t("cal.generate") });
  try {
    const mdFiles = dayFiles.value.filter((f) => f.kind === "md");
    const contents: string[] = [];
    for (const f of mdFiles.slice(0, 15)) {
      try {
        const text = await api.readTextFile(f.path);
        contents.push(`### ${f.path}\n${text.slice(0, 800)}`);
      } catch { /* 跳过 */ }
    }
    const ctx = contents.join("\n\n") || "(无笔记内容)";

    // 规则 = 选中的文件模板全文（可为很长的公司规范/技能文档）或内置默认
    const rules = await currentRules();
    let prompt = rules.replace(/\{\{date\}\}/gi, selectedDate.value);
    if (/\{\{items\}\}/i.test(prompt)) {
      prompt = prompt.replace(/\{\{items\}\}/gi, ctx);
    } else {
      prompt += `\n\n当天编辑的素材：\n${ctx}`;
    }

    const sys = `你是日志生成助手。《生成规则》来自用户配置，可能是一份很长的公司规范或技能文档，必须逐条严格遵守，与用户素材冲突时以规则为准。规则中若已包含输出格式，按格式输出；输出纯 Markdown 正文，不要任何解释或代码块围栏。用户可以通过后续对话继续调整结果。`;

    const result = await api.aiChat(`《生成规则》\n${prompt}`, sys);
    generatedLog.value = result;
    aiMessages.value.push({ role: "assistant", text: t("cal.generated") });
  } catch (e) {
    aiMessages.value.push({ role: "assistant", text: `${t("ai.noKey")}: ${e}` });
  } finally {
    aiRunning.value = false;
  }
}

async function sendAiMessage() {
  const msg = aiInput.value.trim();
  if (!msg || aiRunning.value) return;
  aiInput.value = "";
  aiRunning.value = true;
  aiMessages.value.push({ role: "user", text: msg });
  try {
    const rules = await currentRules();
    const sys = `你是日志调整助手，必须继续遵守《生成规则》。
《生成规则》：
${rules}
${generatedLog.value ? `\n当前生成的日志：\n${generatedLog.value}\n` : ""}
根据用户的要求修改日志内容，直接输出修改后的完整日志，不要解释。`;
    const result = await api.aiChat(msg, sys);
    if (generatedLog.value) {
      generatedLog.value = result; // 已有日志则更新日志
    } else {
      generatedLog.value = result; // 尚未生成则把调整结果作为初稿
    }
    aiMessages.value.push({ role: "assistant", text: t("cal.updated") });
  } catch (e) {
    aiMessages.value.push({ role: "assistant", text: String(e) });
  } finally {
    aiRunning.value = false;
  }
}

async function saveLog() {
  if (!generatedLog.value || !selectedDate.value) return;
  const name = tf("cal.logFileName", { d: selectedDate.value });
  try {
    await editor.openNote(`${name}.md`);
    editor.setContent(generatedLog.value);
    await editor.save();
  } catch {
    // 不存在则创建
    const { useVaultStore } = await import("../../stores/vault");
    const vault = useVaultStore();
    await vault.newNote(null, name, generatedLog.value);
  }
}

// ---- 导航 ----
function shiftMonth(delta: number) {
  const d = new Date(viewYear.value, viewMonth.value - 1 + delta, 1);
  viewYear.value = d.getFullYear();
  viewMonth.value = d.getMonth() + 1;
}
function goToday() {
  const n = new Date();
  viewYear.value = n.getFullYear();
  viewMonth.value = n.getMonth() + 1;
  selectDate(mtimeToDateStr(Math.floor(n.getTime() / 1000)));
}
function selectDate(date: string) {
  selectedDate.value = date;
  generatedLog.value = "";
  aiMessages.value = [];
}
async function refreshFiles() {
  try {
    allFiles.value = await api.listAllFilesWithMtime();
  } catch {
    allFiles.value = [];
  }
}
async function openFile(f: { path: string; kind: string }) {
  if (f.kind === "canvas") {
    ui.canvasPath = f.path;
    ui.view = "canvas";
  } else if (f.kind === "graph") {
    ui.graphPath = f.path;
    ui.view = "graph";
  } else {
    await editor.openNote(f.path);
    ui.view = "editor";
  }
}
function closeCalendar() {
  ui.view = "editor";
}

onMounted(() => {
  refreshFiles();
  const n = new Date();
  selectedDate.value = mtimeToDateStr(Math.floor(n.getTime() / 1000));
  logTplName.value = settings.data.log_template_name || "";
  refreshLogTemplates();
});
</script>

<style scoped>
.calendar-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--background-primary);
  overflow: hidden;
}

/* 头部 */
.cal-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--background-modifier-border);
  background: var(--background-secondary);
  flex-shrink: 0;
}
.cal-nav-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.cal-nav-btn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.cal-title {
  display: flex;
  align-items: baseline;
  gap: 2px;
  margin: 0 4px;
  user-select: none;
}
.cal-y { font-size: 18px; font-weight: 700; color: var(--text-normal); }
.cal-ym { color: var(--text-faint); }
.cal-m { font-size: 16px; font-weight: 600; color: var(--text-normal); }
.cal-today { margin-left: 8px; }
.cal-spacer { flex: 1; }

/* 主体 */
.cal-body {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
}

/* 月历格子 */
.cal-grid-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding: 10px 14px;
  overflow-y: auto;
}
.cal-weekdays {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 3px;
  margin-bottom: 4px;
  flex-shrink: 0;
}
.cal-wd {
  text-align: center;
  font-size: var(--font-ui-smaller);
  color: var(--text-faint);
  padding: 4px 0;
}
.cal-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  grid-template-rows: repeat(6, 1fr);
  gap: 3px;
  flex: 1;
  min-height: 0;
}
.cal-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  padding: 8px 6px;
  border-radius: var(--radius-m);
  border: 1px solid transparent;
  cursor: pointer;
  transition: background var(--anim-fast), border-color var(--anim-fast);
  min-height: 86px;
  text-align: center;
}
.cal-cell:hover { background: var(--background-modifier-hover); }
.cal-cell.is-other-month { opacity: 0.35; }
.cal-cell.is-today { border-color: var(--interactive-accent); }
.cal-cell.is-today .cal-day-num { color: var(--interactive-accent); font-weight: 700; }
.cal-cell.is-selected { background: var(--interactive-accent-hover-alt); border-color: var(--interactive-accent); }
.cal-day-num {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-normal);
  line-height: 1.3;
  margin-bottom: 4px;
}
.cal-stats {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: center;
}
.cal-stat {
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-normal);
  white-space: nowrap;
  font-weight: 500;
}

/* 右侧面板 */
.cal-side {
  width: 300px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--background-modifier-border);
  background: var(--background-secondary);
  overflow: hidden;
}
.cal-file-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 6px;
  min-height: 0;
}
.cal-group { margin-bottom: 6px; }
.cal-group-header {
  font-size: var(--font-ui-smaller);
  color: var(--text-faint);
  padding: 4px 6px 2px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.cal-file-item {
  padding: 4px 8px;
  height: auto;
  gap: 6px;
}
.cal-file-icon { color: var(--text-faint); flex-shrink: 0; }
.cal-file-name {
  font-size: var(--font-ui-size);
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-side-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 24px 12px;
  font-size: var(--font-ui-smaller);
}

/* AI 日志生成器：底部通栏重点区 */
.cal-ai-bar {
  flex-shrink: 0;
  height: 320px;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--background-modifier-border);
  background: var(--background-secondary);
  overflow: hidden;
}
.cal-ai-bar.is-collapsed {
  height: auto;
}
.cal-ai-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  flex-shrink: 0;
}
.cal-ai-head-icon {
  color: var(--text-accent);
}
.cal-ai-head-title {
  color: var(--text-normal);
  font-weight: 600;
  font-size: 14px;
}
.cal-ai-head-sub {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.cal-ai-tpl-btn {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  padding: 0 8px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
}
.cal-ai-tpl-btn:hover { color: var(--text-normal); background: var(--background-modifier-hover); }
.cal-ai-fold {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.cal-ai-fold:hover { background: var(--background-modifier-hover); color: var(--text-normal); }

/* 日志模板管理弹窗 */
.cal-tpl-modal {
  width: min(860px, 92vw);
  height: min(600px, 86vh);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cal-tpl-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.cal-tpl-title {
  font-weight: 600;
  color: var(--text-normal);
  font-size: var(--font-ui-medium);
}
.cal-tpl-dir {
  flex: 1;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-tpl-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 12px;
}
.cal-tpl-side {
  width: 210px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  border-right: 1px solid var(--background-modifier-border);
  padding-right: 8px;
}
.cal-tpl-side-btns {
  display: flex;
  gap: 4px;
  margin-bottom: 4px;
}
.cal-tpl-item {
  gap: 7px;
  flex-shrink: 0;
}
.cal-tpl-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-tpl-side-empty {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  padding: 12px 6px;
  line-height: 1.6;
}
.cal-tpl-editor {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.cal-tpl-edit-head {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.cal-tpl-edit-name {
  flex: 1;
  font-weight: 600;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-tpl-danger:hover {
  color: var(--text-error);
}
.cal-tpl-textarea {
  flex: 1;
  min-height: 0;
  resize: none;
  font-family: var(--font-text);
  font-size: 13px;
  line-height: 1.7;
  padding: 10px 12px;
}
.cal-tpl-edit-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-end;
  flex-shrink: 0;
}
.cal-tpl-vars {
  flex: 1;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.cal-ai-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 14px;
  padding: 4px 16px 12px;
}
.cal-ai-left,
.cal-ai-right {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.cal-ai-right {
  flex: 1.25;
}
.cal-ai-msgs {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 4px 0;
}
.cal-ai-msg {
  font-size: var(--font-ui-size);
  padding: 5px 10px;
  border-radius: var(--radius-m);
  line-height: 1.5;
  max-width: 86%;
  white-space: pre-wrap;
}
.cal-ai-msg.user {
  background: var(--interactive-accent-hover-alt);
  color: var(--text-accent);
  align-self: flex-end;
}
.cal-ai-msg.assistant {
  background: var(--background-modifier-hover);
  color: var(--text-muted);
  align-self: flex-start;
}
.cal-ai-placeholder {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  line-height: 1.7;
}
.cal-ai-result-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.cal-ai-result-input {
  flex: 1;
  width: 100%;
  min-height: 0;
  font-family: var(--font-text);
  font-size: 13px;
  line-height: 1.6;
  resize: none;
}
.cal-ai-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}
.cal-ai-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
}
.cal-ai-gen { flex-shrink: 0; }
.cal-ai-input { flex: 1; height: 32px; font-size: 13px; }
.cal-ai-loading {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-accent);
  font-size: var(--font-ui-size);
  padding: 6px 0;
}
.cal-ai-nofiles {
  padding: 16px;
  text-align: center;
}
</style>
