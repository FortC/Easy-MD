<template>
  <div class="props-panel">
    <div class="emd-panel-header">
      <span>{{ t("pp.title2") }}</span>
      <button class="pp-add" :title="t('pp.addField')" @click="startAddField">
        <Icon name="plus" :size="13" />
      </button>
    </div>

    <div v-if="parseError" class="pp-error">
      {{ t("pp.yamlErr") }}
    </div>

    <div v-if="!parseError && hasFm" class="pp-rows">
      <!-- 别名 -->
      <div class="pp-row">
        <div class="pp-key">{{ t("pp.aliases") }}</div>
        <div class="pp-chips">
          <span v-for="(a, i) in aliases" :key="i" class="pp-chip">
            {{ a }}
            <button class="pp-chip-x" @click="removeAlias(i)">
              <Icon name="x" :size="9" />
            </button>
          </span>
          <input
            v-model="aliasInput"
            class="pp-chip-input"
            :placeholder="t('pp.addAlias')"
            @keydown.enter="addAlias"
          />
        </div>
      </div>
      <!-- 标签 -->
      <div class="pp-row">
        <div class="pp-key">{{ t("pp.tags") }}</div>
        <div class="pp-chips">
          <span v-for="(t, i) in tags" :key="i" class="pp-chip pp-chip-tag">
            #{{ t }}
            <button class="pp-chip-x" @click="removeTag(i)">
              <Icon name="x" :size="9" />
            </button>
          </span>
          <input
            v-model="tagInput"
            class="pp-chip-input"
            :placeholder="t('pp.addTag')"
            @keydown.enter="addTag"
          />
        </div>
      </div>
      <!-- 自定义字段 -->
      <div v-for="f in customFields" :key="f.key" class="pp-row">
        <div class="pp-key" :title="f.key">{{ f.key }}</div>
        <input
          class="pp-value"
          :value="f.value"
          @change="setField(f.key, ($event.target as HTMLInputElement).value)"
        />
        <button class="pp-del" @click="removeField(f.key)">
          <Icon name="x" :size="10" />
        </button>
      </div>
    </div>
    <div v-else-if="!hasFm && editor.isOpen" class="pp-hint">
      {{ t("pp.noFm") }}
      <button class="emd-btn" @click="initFrontmatter">{{ t("pp.add") }}</button>
    </div>
    <div v-else-if="!editor.isOpen" class="pp-hint">{{ t("pp.notOpen") }}</div>

    <InputDialog
      v-if="addingField"
      :title="t('pp.fieldName')"
      :placeholder="t('pp.fieldName')"
      @confirm="addField"
      @cancel="addingField = false"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import YAML from "yaml";
import Icon from "../common/Icon.vue";
import InputDialog from "../common/InputDialog.vue";
import { useEditorStore } from "../../stores/editor";
import { stripFrontmatter } from "../../lib/markdown/renderer";
import { t } from "../../i18n";

const editor = useEditorStore();
const aliasInput = ref("");
const tagInput = ref("");
const addingField = ref(false);
const parseError = ref(false);

const parsed = computed<{ fm: Record<string, unknown> | null }>(() => {
  if (!editor.isOpen) return { fm: null };
  const { fm } = stripFrontmatter(editor.content);
  if (fm === null) {
    parseError.value = false;
    return { fm: null };
  }
  try {
    const obj = YAML.parse(fm) as Record<string, unknown> | null;
    parseError.value = false;
    return { fm: obj && typeof obj === "object" ? obj : {} };
  } catch {
    parseError.value = true;
    return { fm: null };
  }
});

const hasFm = computed(() => parsed.value.fm !== null);

const aliases = computed<string[]>(() => {
  const v = parsed.value.fm?.["aliases"] ?? parsed.value.fm?.["alias"];
  if (typeof v === "string") return [v];
  if (Array.isArray(v)) return v.map(String);
  return [];
});

const tags = computed<string[]>(() => {
  const v = parsed.value.fm?.["tags"] ?? parsed.value.fm?.["tag"];
  if (typeof v === "string")
    return v.split(",").map((s) => s.trim()).filter(Boolean);
  if (Array.isArray(v)) return v.map(String);
  return [];
});

const customFields = computed(() => {
  const fm = parsed.value.fm;
  if (!fm) return [];
  return Object.entries(fm)
    .filter(([k]) => !["aliases", "alias", "tags", "tag"].includes(k))
    .map(([key, v]) => ({
      key,
      value: typeof v === "object" ? JSON.stringify(v) : String(v),
    }));
});

// ---- 写回 ----
function writeFm(mutate: (obj: Record<string, unknown>) => void) {
  const { fm, body } = stripFrontmatter(editor.content);
  let obj: Record<string, unknown> = {};
  if (fm !== null) {
    try {
      obj = (YAML.parse(fm) as Record<string, unknown>) || {};
    } catch {
      return; // 解析失败不允许写
    }
  }
  mutate(obj);
  const yamlText = YAML.stringify(obj).trimEnd();
  const text = `---\n${yamlText}\n---\n${body}`;
  editor.setContent(text);
}

function addAlias() {
  const v = aliasInput.value.trim();
  if (!v) return;
  writeFm((obj) => {
    const arr = Array.isArray(obj["aliases"]) ? [...obj["aliases"] as unknown[]] : aliases.value.slice();
    if (!arr.includes(v)) arr.push(v);
    obj["aliases"] = arr;
    delete obj["alias"];
  });
  aliasInput.value = "";
}

function removeAlias(i: number) {
  writeFm((obj) => {
    const arr = aliases.value.slice();
    arr.splice(i, 1);
    obj["aliases"] = arr;
  });
}

function addTag() {
  const v = tagInput.value.trim().replace(/^#/, "");
  if (!v) return;
  writeFm((obj) => {
    const arr = Array.isArray(obj["tags"]) ? [...obj["tags"] as unknown[]] : tags.value.slice();
    if (!arr.includes(v)) arr.push(v);
    obj["tags"] = arr;
    delete obj["tag"];
  });
  tagInput.value = "";
}

function removeTag(i: number) {
  writeFm((obj) => {
    const arr = tags.value.slice();
    arr.splice(i, 1);
    obj["tags"] = arr;
  });
}

function startAddField() {
  addingField.value = true;
}

function addField(key: string) {
  addingField.value = false;
  writeFm((obj) => {
    if (!(key in obj)) obj[key] = "";
  });
}

function setField(key: string, raw: string) {
  writeFm((obj) => {
    // 尝试保留类型：数字/布尔
    if (raw === "true") obj[key] = true;
    else if (raw === "false") obj[key] = false;
    else if (raw !== "" && !Number.isNaN(Number(raw))) obj[key] = Number(raw);
    else obj[key] = raw;
  });
}

function removeField(key: string) {
  writeFm((obj) => {
    delete obj[key];
  });
}

function initFrontmatter() {
  writeFm(() => {});
}

watch(
  () => editor.openToken,
  () => {
    aliasInput.value = "";
    tagInput.value = "";
  },
);
</script>

<style scoped>
.props-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.pp-add {
  padding: 3px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.pp-add:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.pp-error {
  color: var(--text-error);
  padding: 8px 10px;
  font-size: var(--font-ui-smaller);
}
.pp-rows {
  flex: 1;
  overflow-y: auto;
  padding: 4px 6px;
}
.pp-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 5px 4px;
  border-bottom: 1px solid var(--background-modifier-border);
}
.pp-row:last-child {
  border-bottom: none;
}
.pp-key {
  width: 70px;
  flex-shrink: 0;
  color: var(--text-muted);
  padding-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pp-chips {
  flex: 1;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}
.pp-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--background-modifier-hover);
  border-radius: var(--radius-m);
  padding: 2px 6px;
  font-size: var(--font-ui-smaller);
}
.pp-chip-tag {
  background: var(--tag-background);
  color: var(--tag-color);
}
.pp-chip-x {
  display: flex;
  color: var(--text-muted);
}
.pp-chip-x:hover {
  color: var(--text-error);
}
.pp-chip-input {
  flex: 1;
  min-width: 100px;
  background: none;
  border: none;
  height: 24px;
  font-size: var(--font-ui-smaller);
}
.pp-chip-input:focus {
  outline: none;
}
.pp-value {
  flex: 1;
  background: none;
  border: 1px solid transparent;
  border-radius: var(--radius-s);
  height: 26px;
  padding: 0 6px;
  color: var(--text-normal);
}
.pp-value:hover,
.pp-value:focus {
  border-color: var(--background-modifier-border);
}
.pp-del {
  color: var(--text-faint);
  padding: 4px;
  border-radius: var(--radius-s);
  visibility: hidden;
}
.pp-row:hover .pp-del {
  visibility: visible;
}
.pp-del:hover {
  color: var(--text-error);
}
.pp-hint {
  color: var(--text-faint);
  padding: 16px 10px;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: center;
}
</style>
