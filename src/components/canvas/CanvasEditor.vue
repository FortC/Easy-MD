<template>
  <div
    class="canvas-editor"
    :class="{ 'is-linking': linking || armed !== null }"
    @wheel.prevent="onWheel"
    @pointerdown.self="startPan"
  >
    <!-- 工具栏 -->
    <div class="ce-toolbar">
      <div class="ce-title" :title="ui.canvasPath">
        <Icon name="layout-grid" :size="13" class="ce-title-icon" />
        <span>{{ canvasName }}</span>
      </div>
      <div class="ce-group">
        <button class="ce-tbtn" :title="t('cv.addNode')" @click="addNodeCard">
          <Icon name="circle" :size="12" /> {{ t("cv.node") }}
        </button>
        <button class="ce-tbtn" :title="t('cv.addText')" @click="addTextCard">
          <Icon name="plus" :size="12" /> {{ t("cv.text") }}
        </button>
        <button class="ce-tbtn" :title="t('cv.addNote')" @click="addNoteCard">
          <Icon name="file-text" :size="12" /> {{ t("cv.note") }}
        </button>
        <button class="ce-tbtn" :title="t('cv.addImage')" @click="addImageCard">
          <Icon name="image" :size="12" /> {{ t("cv.image") }}
        </button>
      </div>
      <span class="ce-spacer" />
      <div class="ce-zoom-group" :title="t('cv.zoomTitle')">
        <button class="ce-zbtn" :title="t('cv.zoomOut2')" @click="zoomBy(1 / 1.2)">
          <Icon name="minus" :size="12" />
        </button>
        <span class="ce-zoom-num" @dblclick="zoomSet(1)">{{ Math.round(view.scale * 100) }}%</span>
        <button class="ce-zbtn" :title="t('cv.zoomIn2')" @click="zoomBy(1.2)">
          <Icon name="plus" :size="12" />
        </button>
      </div>
      <button class="ce-tbtn" :title="t('cv.fitAll')" @click="fitView">{{ t("cv.fit") }}</button>
      <button class="ce-tbtn" :title="t('cv.backEd')" @click="closeCanvas">
        <Icon name="x" :size="12" /> {{ t("c.close") }}
      </button>
    </div>

    <!-- 画布区 -->
    <div ref="viewport" class="ce-viewport">
      <div
        class="ce-world"
        :style="{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }"
      >
        <!-- 连线层 -->
        <svg class="ce-edges">
          <template v-for="e in data.edges" :key="e.id">
            <path
              :d="edgePath(e)"
              class="ce-edge"
              :class="{ 'is-selected': selectedEdge === e.id }"
              @pointerdown.stop="selectEdge(e.id)"
            />
            <!-- 选中连线：中点显示删除按钮 -->
            <g
              v-if="selectedEdge === e.id"
              class="ce-edge-del"
              @pointerdown.stop="removeEdge(e.id)"
            >
              <circle :cx="edgeMid(e).x" :cy="edgeMid(e).y" r="11" />
              <text :x="edgeMid(e).x" :y="edgeMid(e).y + 5" text-anchor="middle">×</text>
            </g>
          </template>
        </svg>

        <!-- 卡片 -->
        <div
          v-for="n in data.nodes"
          :key="n.id"
          class="ce-card"
          :class="[
            'ce-' + n.type,
            {
              'is-selected': selectedNode === n.id,
              'is-editing': editingNode === n.id,
              'is-armed': armed !== null && armed.id === n.id,
            },
          ]"
          :style="{ left: n.x + 'px', top: n.y + 'px', width: n.width + 'px', height: n.height + 'px' }"
          @pointerdown.stop="startMove(n, $event)"
        >
          <!-- 节点卡片：无标题栏，整卡可拖，双击编辑，悬停显示删除 -->
          <div v-if="n.type === 'node'" class="ce-node-body" @dblclick.stop="startNodeEdit(n)">
            <input
              v-if="editingNode === n.id"
              class="ce-node-input"
              :value="n.text"
              :placeholder="t('cv.nodePh')"
              @input="onTextInput(n, ($event.target as HTMLInputElement).value)"
              @pointerdown.stop
              @blur="editingNode = ''"
              @keydown.enter="($event.target as HTMLInputElement).blur()"
            />
            <span v-else class="ce-node-label">{{ n.text || t("cv.node") }}</span>
            <button class="ce-card-del ce-node-del" :title="t('cv.delNode')" @pointerdown.stop @click="removeNode(n.id)">
              <Icon name="x" :size="10" />
            </button>
          </div>

          <!-- 其它卡片：标题栏（拖拽/打开） -->
          <div v-else class="ce-card-head" @dblclick="n.type === 'file' && openNote(n.file!)">
            <Icon :name="n.type === 'file' ? 'file-text' : n.type === 'image' ? 'image' : 'pencil'" :size="12" />
            <span class="ce-card-title" :title="cardTitle(n)">
              {{ cardTitle(n) }}
            </span>
            <button class="ce-card-del" :title="t('cv.delCard')" @pointerdown.stop @click="removeNode(n.id)">
              <Icon name="x" :size="11" />
            </button>
          </div>

          <!-- 文本卡片 -->
          <textarea
            v-if="n.type === 'text'"
            class="ce-text-input"
            :value="n.text"
            :placeholder="t('cv.textPh')"
            @input="onTextInput(n, ($event.target as HTMLTextAreaElement).value)"
            @pointerdown.stop
          />

          <!-- 笔记卡片 -->
          <template v-if="n.type === 'file'">
            <div v-if="editingNode === n.id" class="ce-note-edit">
              <textarea
                class="ce-text-input"
                :value="n._content"
                @input="onNoteEdit(n, ($event.target as HTMLTextAreaElement).value)"
                @pointerdown.stop
              />
              <div class="ce-edit-hint">{{ t("cv.editHint") }}</div>
            </div>
            <div
              v-else
              class="ce-note-render markdown-rendered"
              @dblclick.stop="startNoteEdit(n)"
              v-html="n._html"
            />
          </template>

          <!-- 图片卡片 -->
          <img
            v-if="n.type === 'image'"
            class="ce-image"
            :src="assetUrlFor(n.file!)"
            draggable="false"
            @dblclick="openNote(n.file!)"
          />

          <!-- 四边连线锚点（文本卡片不参与连线，不显示） -->
          <template v-if="n.type !== 'text'">
            <div
              v-for="s in sides"
              :key="s"
              class="ce-anchor"
              :class="'ce-anchor-' + s"
              :title="tf('cv.fromSide', { side: sideName(s) })"
              @pointerdown.stop="startLink(n, s, $event)"
            />
          </template>
          <!-- 缩放手柄 -->
          <div class="ce-resize" @pointerdown.stop="startResize(n, $event)" />
        </div>

        <!-- 新建连线时的跟随线（必须画在 world 内，与卡片同一坐标系） -->
        <svg v-if="linking" class="ce-linking-svg">
          <path :d="linkingPath" class="ce-edge is-linking" />
        </svg>
      </div>

      <!-- 待连线提示 -->
      <div v-if="armed" class="ce-armed-hint">
        {{ t("cv.armed") }}
      </div>
    </div>

    <!-- 笔记选择弹窗 -->
    <transition name="emd-fade">
      <div v-if="pickingNote" class="emd-modal-bg" @click.self="pickingNote = false">
      <div class="emd-modal ce-picker">
        <input v-model="pickQuery" type="text" :placeholder="t('cv.pickNote')" />
        <div class="ce-picker-list">
          <div
            v-for="n in pickResults"
            :key="n.path"
            class="emd-tree-item"
            @click="chosenNote(n.path)"
          >
            <Icon name="file-text" :size="13" />
            <span>{{ n.title }}</span>
            <span class="ce-picker-path">{{ n.path }}</span>
          </div>
        </div>
      </div>
    </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import Icon from "../common/Icon.vue";
import { api } from "../../ipc/tauri";
import { useUiStore } from "../../stores/ui";
import { useVaultStore } from "../../stores/vault";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useSettingsStore } from "../../stores/settings";
import { renderMarkdown, emdAssetUrl } from "../../lib/markdown/renderer";
import { resolveTarget } from "../../lib/markdown/links";
import { t, tf } from "../../i18n";

// ---- Obsidian 兼容的 .canvas JSON 结构 ----
interface CanvasNode {
  id: string;
  type: "file" | "text" | "image" | "node";
  file?: string;
  text?: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** 视图层缓存（不序列化） */
  _html?: string;
  _content?: string;
  _timer?: ReturnType<typeof setTimeout>;
}
interface CanvasEdge {
  id: string;
  fromNode: string;
  fromSide: "top" | "right" | "bottom" | "left";
  toNode: string;
  toSide: "top" | "right" | "bottom" | "left";
}

const ui = useUiStore();
const vault = useVaultStore();
const indexStore = useNotesIndexStore();
const settings = useSettingsStore();

const viewport = ref<HTMLElement>();
const data = reactive<{ nodes: CanvasNode[]; edges: CanvasEdge[] }>({
  nodes: [],
  edges: [],
});
const view = reactive({ x: 0, y: 0, scale: 1 });
const selectedNode = ref("");
const selectedEdge = ref("");
const editingNode = ref("");
const sides = ["top", "right", "bottom", "left"] as const;

const canvasName = computed(() =>
  ui.canvasPath.split("/").pop()?.replace(/\.canvas$/i, "") || t("cv.untitled"),
);

// 拖拽状态
let drag: {
  kind: "move" | "resize" | "pan";
  node?: CanvasNode;
  startX: number;
  startY: number;
  orig: { x: number; y: number; w: number; h: number; vx: number; vy: number };
} | null = null;

// 连线状态
const linking = ref(false);
const linkingPath = ref("");
/** 点击锚点（未拖动）后的"待连线"状态：再点一张卡片即完成连线 */
const armed = ref<{ id: string; side: string } | null>(null);

const pickingNote = ref(false);
const pickQuery = ref("");
const pickResults = computed(() => {
  const q = pickQuery.value.trim().toLowerCase();
  const list = indexStore.notes;
  if (!q) return list.slice(0, 30);
  return list
    .filter(
      (n) =>
        n.title.toLowerCase().includes(q) ||
        n.path.toLowerCase().includes(q),
    )
    .slice(0, 30);
});

let saveTimer: ReturnType<typeof setTimeout> | null = null;
/** 自己写盘后，忽略监听器回声的窗口期（防止拖拽/连线进行中被重载打断） */
let suppressReloadUntil = 0;

/** 立即写盘（卸载/切换视图前必须调用，避免防抖丢数据） */
async function saveNow() {
  if (!ui.canvasPath) return;
  const out = {
    nodes: data.nodes.map(({ id, type, file, text, x, y, width, height }) => {
      const o: Record<string, unknown> = { id, type, x, y, width, height };
      if (file !== undefined) o.file = file;
      if (text !== undefined) o.text = text;
      return o;
    }),
    edges: data.edges,
  };
  try {
    await api.writeTextFile(ui.canvasPath, JSON.stringify(out, null, 2));
    suppressReloadUntil = Date.now() + 1500;
  } catch {
    /* 磁盘异常时静默，避免卸载路径报错 */
  }
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void saveNow();
  }, 400);
}

// ---------- 加载 / 保存 ----------
onMounted(async () => {
  await load();
  window.addEventListener("emd-canvas-changed", onExternalChange as EventListener);
  window.addEventListener("keydown", onEsc);
  fitView();
});

onUnmounted(() => {
  window.removeEventListener("emd-canvas-changed", onExternalChange as EventListener);
  window.removeEventListener("keydown", onEsc);
  // 卸载前把防抖中的修改立即落盘
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = null;
    void saveNow();
  }
});

function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape" && armed.value) armed.value = null;
  // Delete/Backspace 删除选中的连线（输入框内不拦截）
  const t = e.target as HTMLElement;
  if ((e.key === "Delete" || e.key === "Backspace") && selectedEdge.value) {
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;
    removeEdge(selectedEdge.value);
  }
}

async function load() {
  if (!ui.canvasPath) return;
  try {
    const raw = await api.readTextFile(ui.canvasPath);
    const obj = JSON.parse(raw) as { nodes?: CanvasNode[]; edges?: CanvasEdge[] };
    data.nodes = (obj.nodes || []).map((n) => ({ ...n }));
    data.edges = (obj.edges || []).map((e) => ({ ...e }));
    for (const n of data.nodes) if (n.type === "file") await hydrateNote(n);
  } catch {
    data.nodes = [];
    data.edges = [];
  }
}

async function hydrateNote(n: CanvasNode) {
  if (!n.file) return;
  try {
    const raw = await api.readTextFile(n.file);
    n._content = raw;
    n._html = renderMarkdown(raw, {
      resolveNote: (t) => resolveTarget(t, indexStore.notes).path,
      attachmentsDir: settings.data.attachments_dir,
      sourcePath: n.file,
    });
  } catch {
    n._html = `<div class="ce-missing">${t("cv.readFail2")}${n.file}</div>`;
  }
}

function onExternalChange(e: CustomEvent<string>) {
  if (e.detail !== ui.canvasPath) return;
  if (Date.now() < suppressReloadUntil) return; // 自己写盘的回声，跳过重载
  load();
}

// ---------- 视图：平移 / 缩放 ----------
function onWheel(e: WheelEvent) {
  const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
  zoomAt(e.clientX, e.clientY, factor);
}

function zoomAt(cx: number, cy: number, factor: number) {
  const rect = viewport.value?.getBoundingClientRect();
  if (!rect) return;
  const px = cx - rect.left;
  const py = cy - rect.top;
  const ns = Math.min(3, Math.max(0.15, view.scale * factor));
  const k = ns / view.scale;
  view.x = px - (px - view.x) * k;
  view.y = py - (py - view.y) * k;
  view.scale = ns;
}

function zoomBy(f: number) {
  const rect = viewport.value?.getBoundingClientRect();
  if (rect) zoomAt(rect.left + rect.width / 2, rect.top + rect.height / 2, f);
}

function zoomSet(s: number) {
  const rect = viewport.value?.getBoundingClientRect();
  if (rect) zoomAt(rect.left + rect.width / 2, rect.top + rect.height / 2, s / view.scale);
}

function startPan(e: PointerEvent) {
  armed.value = null; // 点空白取消待连线
  selectedNode.value = "";
  selectedEdge.value = "";
  drag = {
    kind: "pan",
    startX: e.clientX,
    startY: e.clientY,
    orig: { x: 0, y: 0, w: 0, h: 0, vx: view.x, vy: view.y },
  };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragUp);
}

function fitView() {
  if (data.nodes.length === 0) {
    view.x = 0;
    view.y = 0;
    view.scale = 1;
    return;
  }
  const minX = Math.min(...data.nodes.map((n) => n.x));
  const minY = Math.min(...data.nodes.map((n) => n.y));
  const maxX = Math.max(...data.nodes.map((n) => n.x + n.width));
  const maxY = Math.max(...data.nodes.map((n) => n.y + n.height));
  const rect = viewport.value?.getBoundingClientRect();
  if (!rect) return;
  const scale = Math.min(1, (rect.width - 80) / (maxX - minX || 1), (rect.height - 80) / (maxY - minY || 1));
  view.scale = Math.max(0.15, scale);
  view.x = (rect.width - (maxX - minX) * view.scale) / 2 - minX * view.scale;
  view.y = (rect.height - (maxY - minY) * view.scale) / 2 - minY * view.scale;
}

// ---------- 卡片拖动 / 缩放 ----------
function startMove(n: CanvasNode, e: PointerEvent) {
  // 待连线模式：点击目标卡片完成连线
  if (armed.value) {
    const a = armed.value;
    armed.value = null;
    if (n.id !== a.id) {
      connectTo(a.id, a.side, n.x + n.width / 2, n.y + n.height / 2);
    }
    return;
  }
  if (editingNode.value === n.id) return; // 编辑中不拖动
  selectedNode.value = n.id;
  selectedEdge.value = "";
  editingNode.value = "";
  drag = {
    kind: "move",
    node: n,
    startX: e.clientX,
    startY: e.clientY,
    orig: { x: n.x, y: n.y, w: 0, h: 0, vx: 0, vy: 0 },
  };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragUp);
}

function startResize(n: CanvasNode, e: PointerEvent) {
  drag = {
    kind: "resize",
    node: n,
    startX: e.clientX,
    startY: e.clientY,
    orig: { x: n.x, y: n.y, w: n.width, h: n.height, vx: 0, vy: 0 },
  };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragUp);
}

function onDragMove(e: PointerEvent) {
  if (!drag) return;
  // 节点对象若被重载替换（外部改动），中断本次拖拽避免"拖不动"
  if (drag.node && !data.nodes.includes(drag.node)) {
    drag = null;
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup", onDragUp);
    return;
  }
  const dx = (e.clientX - drag.startX) / view.scale;
  const dy = (e.clientY - drag.startY) / view.scale;
  if (drag.kind === "pan") {
    view.x = drag.orig.vx + (e.clientX - drag.startX);
    view.y = drag.orig.vy + (e.clientY - drag.startY);
  } else if (drag.kind === "move" && drag.node) {
    drag.node.x = Math.round(drag.orig.x + dx);
    drag.node.y = Math.round(drag.orig.y + dy);
  } else if (drag.kind === "resize" && drag.node) {
    drag.node.width = Math.max(160, Math.round(drag.orig.w + dx));
    drag.node.height = Math.max(100, Math.round(drag.orig.h + dy));
  }
}

function onDragUp() {
  if (drag && (drag.kind === "move" || drag.kind === "resize")) scheduleSave();
  drag = null;
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", onDragUp);
}

// ---------- 连线 ----------
function anchorPoint(n: CanvasNode, side: string): { x: number; y: number } {
  switch (side) {
    case "top":
      return { x: n.x + n.width / 2, y: n.y };
    case "bottom":
      return { x: n.x + n.width / 2, y: n.y + n.height };
    case "left":
      return { x: n.x, y: n.y + n.height / 2 };
    default:
      return { x: n.x + n.width, y: n.y + n.height / 2 };
  }
}

function edgePath(e: CanvasEdge): string {
  const from = data.nodes.find((n) => n.id === e.fromNode);
  const to = data.nodes.find((n) => n.id === e.toNode);
  if (!from || !to) return "";
  const a = anchorPoint(from, e.fromSide);
  const b = anchorPoint(to, e.toSide);
  // 贝塞尔：控制点沿边法线方向
  const off = (p: { x: number; y: number }, side: string) => {
    const k = 60;
    if (side === "top") return { x: p.x, y: p.y - k };
    if (side === "bottom") return { x: p.x, y: p.y + k };
    if (side === "left") return { x: p.x - k, y: p.y };
    return { x: p.x + k, y: p.y };
  };
  const c1 = off(a, e.fromSide);
  const c2 = off(b, e.toSide);
  return `M ${a.x} ${a.y} C ${c1.x} ${c1.y}, ${c2.x} ${c2.y}, ${b.x} ${b.y}`;
}

/** 连线交互：拖拽连线 + 点击-点击连线两种方式 */
function startLink(n: CanvasNode, side: string, e: PointerEvent) {
  e.preventDefault();
  e.stopPropagation();
  armed.value = null;
  const anchorEl = e.currentTarget as HTMLElement;
  const sourceId = n.id;
  const startX = e.clientX;
  const startY = e.clientY;
  let moved = false;
  linking.value = true;
  try {
    anchorEl.setPointerCapture(e.pointerId);
  } catch {
    /* 捕获失败时仍有 window 兜底监听 */
  }
  const anchorOfSource = () => {
    // 始终按 id 从当前数据取节点，避免重载后引用失效
    const src = data.nodes.find((x) => x.id === sourceId);
    return src ? anchorPoint(src, side) : null;
  };
  const move = (ev: PointerEvent) => {
    if (Math.hypot(ev.clientX - startX, ev.clientY - startY) > 4) moved = true;
    if (!moved) return;
    const rect = viewport.value?.getBoundingClientRect();
    const a = anchorOfSource();
    if (!rect || !a) return;
    const b = {
      x: (ev.clientX - rect.left - view.x) / view.scale,
      y: (ev.clientY - rect.top - view.y) / view.scale,
    };
    linkingPath.value = `M ${a.x} ${a.y} L ${b.x} ${b.y}`;
  };
  const finish = (ev: PointerEvent, cancelled: boolean) => {
    cleanup();
    linking.value = false;
    if (cancelled) return;
    if (!moved) {
      // 点击（未拖动）→ 进入"待连线"模式，等用户点击目标卡片
      armed.value = { id: sourceId, side };
      return;
    }
    const rect = viewport.value?.getBoundingClientRect();
    if (!rect) return;
    const wx = (ev.clientX - rect.left - view.x) / view.scale;
    const wy = (ev.clientY - rect.top - view.y) / view.scale;
    connectTo(sourceId, side, wx, wy);
  };
  const up = (ev: PointerEvent) => finish(ev, false);
  const cancel = () => finish(new PointerEvent("pointerup", { clientX: -1, clientY: -1 }), true);
  function cleanup() {
    anchorEl.removeEventListener("pointermove", move);
    anchorEl.removeEventListener("pointerup", up);
    anchorEl.removeEventListener("pointercancel", cancel);
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  }
  anchorEl.addEventListener("pointermove", move);
  anchorEl.addEventListener("pointerup", up);
  anchorEl.addEventListener("pointercancel", cancel);
  // 双保险：指针捕获不可用时仍可拖拽
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

/** 在 (wx, wy) 找目标卡片并创建连线 */
function connectTo(sourceId: string, side: string, wx: number, wy: number) {
  const target = data.nodes.find(
    (t) =>
      t.id !== sourceId &&
      wx >= t.x && wx <= t.x + t.width && wy >= t.y && wy <= t.y + t.height,
  );
  if (target) {
    data.edges.push({
      id: crypto.randomUUID(),
      fromNode: sourceId,
      fromSide: side as CanvasEdge["fromSide"],
      toNode: target.id,
      toSide: nearestSide(target, wx, wy),
    });
    scheduleSave();
  }
}

function nearestSide(n: CanvasNode, wx: number, wy: number): CanvasEdge["toSide"] {
  const dists = [
    { s: "top" as const, d: Math.abs(wy - n.y) },
    { s: "bottom" as const, d: Math.abs(n.y + n.height - wy) },
    { s: "left" as const, d: Math.abs(wx - n.x) },
    { s: "right" as const, d: Math.abs(n.x + n.width - wx) },
  ].sort((a, b) => a.d - b.d);
  return dists[0].s;
}

function selectEdge(id: string) {
  selectedEdge.value = id;
  selectedNode.value = "";
}

/** 连线中点（删除按钮位置） */
function edgeMid(e: CanvasEdge): { x: number; y: number } {
  const from = data.nodes.find((n) => n.id === e.fromNode);
  const to = data.nodes.find((n) => n.id === e.toNode);
  if (!from || !to) return { x: 0, y: 0 };
  const a = anchorPoint(from, e.fromSide);
  const b = anchorPoint(to, e.toSide);
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

function removeEdge(id: string) {
  data.edges = data.edges.filter((e) => e.id !== id);
  selectedEdge.value = "";
  scheduleSave();
}

// ---------- 添加 / 删除卡片 ----------
function centerOfView(): { x: number; y: number } {
  const rect = viewport.value?.getBoundingClientRect();
  if (!rect) return { x: 0, y: 0 };
  return {
    x: (rect.width / 2 - view.x) / view.scale - 180,
    y: (rect.height / 2 - view.y) / view.scale - 100,
  };
}

function addTextCard() {
  const c = centerOfView();
  data.nodes.push({
    id: crypto.randomUUID(),
    type: "text",
    text: "",
    x: Math.round(c.x),
    y: Math.round(c.y),
    width: 360,
    height: 200,
  });
  scheduleSave();
}

/** 节点卡片：紧凑单行标签，用于连线/组织结构 */
function addNodeCard() {
  const c = centerOfView();
  data.nodes.push({
    id: crypto.randomUUID(),
    type: "node",
    text: t("cv.node"),
    x: Math.round(c.x),
    y: Math.round(c.y),
    width: 160,
    height: 44,
  });
  scheduleSave();
}

function sideName(s: string): string {
  return { top: t("cv.sideTop"), bottom: t("cv.sideBottom"), left: t("cv.sideLeft"), right: t("cv.sideRight") }[s] || s;
}

/** 节点双击进入编辑并聚焦 */
function startNodeEdit(n: CanvasNode) {
  editingNode.value = n.id;
  setTimeout(() => {
    const el = document.querySelector<HTMLInputElement>(".ce-node-input");
    el?.focus();
    el?.select();
  }, 30);
}

function addNoteCard() {
  pickingNote.value = true;
}

async function chosenNote(path: string) {
  pickingNote.value = false;
  const c = centerOfView();
  const n: CanvasNode = {
    id: crypto.randomUUID(),
    type: "file",
    file: path,
    x: Math.round(c.x),
    y: Math.round(c.y),
    width: 420,
    height: 320,
  };
  data.nodes.push(n);
  await hydrateNote(n);
  scheduleSave();
}

async function addImageCard() {
  const file = await openDialog({
    multiple: false,
    filters: [{ name: t("cv.imageFilter"), extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"] }],
  });
  if (typeof file !== "string" || !file) return;
  // 读文件走 Rust（WebView 禁止 fetch file://）
  const ext = (file.split(".").pop() || "png").toLowerCase();
  try {
    const bytes = await api.readExternalBinary(file);
    const rel = await api.saveImage(bytes, ext);
    const c = centerOfView();
    data.nodes.push({
      id: crypto.randomUUID(),
      type: "image",
      file: rel,
      x: Math.round(c.x),
      y: Math.round(c.y),
      width: 420,
      height: 300,
    });
    scheduleSave();
  } catch (e) {
    alert(`${t("cv.imgFail")}: ${e}`);
  }
}

function removeNode(id: string) {
  data.nodes = data.nodes.filter((n) => n.id !== id);
  data.edges = data.edges.filter((e) => e.fromNode !== id && e.toNode !== id);
  scheduleSave();
}

// ---------- 卡片内容编辑 ----------
let textTimer: ReturnType<typeof setTimeout> | null = null;

function onTextInput(n: CanvasNode, v: string) {
  n.text = v;
  if (textTimer) clearTimeout(textTimer);
  textTimer = setTimeout(() => scheduleSave(), 500);
}

function startNoteEdit(n: CanvasNode) {
  editingNode.value = n.id;
}

function onNoteEdit(n: CanvasNode, v: string) {
  // 双向绑定：卡片内编辑直接写回 md 文件（防抖）
  n._content = v;
  if (n._timer) clearTimeout(n._timer);
  n._timer = setTimeout(async () => {
    if (!n.file) return;
    await api.writeTextFile(n.file, v);
    n._html = renderMarkdown(v, {
      resolveNote: (t) => resolveTarget(t, indexStore.notes).path,
      attachmentsDir: settings.data.attachments_dir,
      sourcePath: n.file,
    });
  }, 600);
}

function cardTitle(n: CanvasNode): string {
  if (n.type === "file") return n.file?.split("/").pop()?.replace(/\.md$/i, "") || t("cv.note");
  if (n.type === "image") return n.file?.split("/").pop() || t("cv.image");
  return t("cv.text");
}

function assetUrlFor(rel: string): string {
  return emdAssetUrl(rel);
}

async function openNote(path: string) {
  await vault.refreshParents(path);
  const { useEditorStore } = await import("../../stores/editor");
  await useEditorStore().openNote(path);
  ui.view = "editor";
}

function closeCanvas() {
  ui.view = "editor";
}
</script>

<style scoped>
.canvas-editor {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--background-primary);
  position: relative;
}
.ce-toolbar {
  height: 38px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 10px 0 16px;
  border-bottom: 1px solid var(--background-modifier-border);
  background: var(--background-secondary);
  flex-shrink: 0;
  z-index: 10;
}
.ce-title {
  display: flex;
  align-items: center;
  gap: 7px;
  font-weight: 600;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ce-title-icon {
  color: var(--text-faint);
}
.ce-group {
  display: flex;
  gap: 2px;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 2px;
}
.ce-tbtn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 10px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: var(--font-ui-size);
  transition: background var(--anim-fast), color var(--anim-fast);
}
.ce-tbtn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ce-spacer {
  flex: 1;
}
.ce-zoom-group {
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 2px;
}
.ce-zbtn {
  width: 26px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.ce-zbtn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ce-zoom-num {
  min-width: 44px;
  text-align: center;
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
  cursor: pointer;
  user-select: none;
}
.ce-viewport {
  flex: 1;
  position: relative;
  overflow: hidden;
  cursor: grab;
  background-image: radial-gradient(
    circle,
    var(--background-modifier-border) 1px,
    transparent 1px
  );
  background-size: 24px 24px;
}
.ce-viewport:active {
  cursor: grabbing;
}
.ce-world {
  position: absolute;
  top: 0;
  left: 0;
  transform-origin: 0 0;
}
.ce-edges {
  position: absolute;
  top: 0;
  left: 0;
  /* 显式大画布：不依赖容器尺寸，坐标永远 1:1 映射 */
  width: 20000px;
  height: 20000px;
  overflow: visible;
  pointer-events: none;
}
.ce-edge {
  fill: none;
  stroke: var(--text-muted);
  stroke-width: 2.5;
  pointer-events: stroke;
  cursor: pointer;
}
.ce-edge.is-selected {
  stroke: var(--interactive-accent);
}
.ce-edge.is-linking {
  stroke-dasharray: 6 4;
  pointer-events: none;
}
.ce-linking-svg {
  position: absolute;
  top: 0;
  left: 0;
  width: 20000px;
  height: 20000px;
  overflow: visible;
  pointer-events: none;
}

/* 拖线/待连线时：所有卡片都显示锚点，方便对准 */
.is-linking .ce-card .ce-anchor {
  opacity: 0.95;
}
.is-linking .ce-viewport,
.canvas-editor.is-linking .ce-viewport {
  cursor: crosshair;
}

/* 连线删除按钮 */
.ce-edge-del {
  cursor: pointer;
}
.ce-edge-del circle {
  fill: var(--background-secondary);
  stroke: var(--background-modifier-border);
  stroke-width: 1;
}
.ce-edge-del text {
  fill: var(--text-muted);
  font-size: 15px;
  font-weight: 700;
}
.ce-edge-del:hover circle {
  stroke: var(--text-error);
}
.ce-edge-del:hover text {
  fill: var(--text-error);
}

/* ---------- 卡片 ---------- */
.ce-card {
  position: absolute;
  background: var(--background-primary);
  border: 1.5px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: var(--shadow-l1);
  cursor: move;
  user-select: none;
  transition: border-color var(--anim-fast);
}
.ce-card.is-selected {
  border-color: var(--interactive-accent);
}
.ce-card-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  background: var(--background-secondary);
  border-bottom: 1px solid var(--background-modifier-border);
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
  flex-shrink: 0;
}
.ce-card-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-normal);
}
.ce-card-del {
  opacity: 0;
  color: var(--text-muted);
  padding: 2px;
}
.ce-card:hover .ce-card-del {
  opacity: 1;
}
.ce-card-del:hover {
  color: var(--text-error);
}
.ce-text-input {
  flex: 1;
  resize: none;
  border: none;
  background: none;
  padding: 10px 12px;
  font-family: var(--font-text);
  font-size: 14px;
  line-height: 1.5;
  color: var(--text-normal);
  overflow-y: auto;
  user-select: text;
}
.ce-text-input:focus {
  outline: none;
}
.ce-note-render {
  flex: 1;
  overflow-y: auto;
  padding: 10px 14px;
  font-size: 13px;
  cursor: default;
  user-select: text;
}
.ce-note-render :deep(p) {
  margin: 0.3em 0 0.6em;
}
.ce-note-render :deep(h1),
.ce-note-render :deep(h2),
.ce-note-render :deep(h3) {
  margin: 0.8em 0 0.4em;
  font-size: 1.1em;
}
.ce-note-edit {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.ce-edit-hint {
  padding: 4px 10px;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  border-top: 1px solid var(--background-modifier-border);
  flex-shrink: 0;
}
.ce-image {
  flex: 1;
  object-fit: contain;
  padding: 8px;
  min-height: 0;
  cursor: default;
}
.ce-missing {
  color: var(--text-error);
  padding: 12px;
  font-size: var(--font-ui-smaller);
}

/* ---------- 节点卡片 ---------- */
.ce-node-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 18px;
  min-height: 0;
  position: relative;
}
.ce-node-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: move;
}
.ce-node-input {
  width: 100%;
  background: none;
  border: none;
  border-bottom: 1px dashed var(--background-modifier-border-hover);
  color: var(--text-normal);
  font-size: 13px;
  font-weight: 600;
  text-align: center;
  user-select: text;
}
.ce-node-input:focus {
  outline: none;
  border-bottom-color: var(--interactive-accent);
}
.ce-node-del {
  position: absolute;
  top: 1px;
  right: 1px;
  padding: 2px;
}
.ce-card.ce-node {
  background: var(--background-secondary-alt);
}

/* ---------- 锚点 / 缩放柄（全部在卡片边界内侧，不受 overflow:hidden 裁剪） ---------- */
.ce-anchor {
  position: absolute;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  /* 20px 命中区，中心 10px 可见圆点 */
  background: radial-gradient(
    circle at center,
    var(--interactive-accent) 0 5px,
    transparent 5px
  );
  opacity: 0;
  transition: opacity var(--anim-fast), transform var(--anim-fast);
  cursor: crosshair;
  z-index: 7;
}
.ce-card:hover .ce-anchor,
.ce-card.is-armed .ce-anchor {
  opacity: 0.95;
}
.ce-anchor:hover {
  opacity: 1;
  transform: scale(1.3);
}
.ce-anchor-top { top: 0; left: calc(50% - 10px); }
.ce-anchor-bottom { bottom: 0; left: calc(50% - 10px); }
.ce-anchor-left { left: 0; top: calc(50% - 10px); }
.ce-anchor-right { right: 0; top: calc(50% - 10px); }
.ce-resize {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 16px;
  height: 16px;
  cursor: nwse-resize;
  z-index: 7;
  opacity: 0;
  background: radial-gradient(circle at 85% 85%, var(--interactive-accent) 0 4px, transparent 4px);
}
.ce-card:hover .ce-resize {
  opacity: 0.9;
}
/* 连线待选状态：源卡片高亮 */
.ce-card.is-armed {
  border-color: var(--interactive-accent);
  box-shadow: 0 0 0 2px var(--interactive-accent-hover-alt), var(--shadow-l1);
}
.ce-armed-hint {
  position: absolute;
  bottom: 18px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--background-secondary);
  border: 1px solid var(--interactive-accent);
  color: var(--text-normal);
  border-radius: var(--radius-m);
  padding: 6px 14px;
  font-size: var(--font-ui-size);
  z-index: 40;
  box-shadow: var(--shadow-l2);
  pointer-events: none;
}

/* ---------- 笔记选择 ---------- */
.ce-picker {
  width: 460px;
  padding: 12px;
  gap: 10px;
}
.ce-picker-list {
  margin-top: 10px;
  max-height: 320px;
  overflow-y: auto;
}
.ce-picker-path {
  margin-left: auto;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
