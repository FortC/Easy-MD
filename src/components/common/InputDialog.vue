<template>
  <transition name="emd-fade">
    <div class="emd-modal-bg" @click.self="cancel">
      <div class="emd-modal input-dialog">
        <div class="id-title">{{ title }}</div>
        <input
          ref="inputRef"
          v-model="value"
          type="text"
          :placeholder="placeholder"
          @keydown.enter="confirm"
          @keydown.esc="cancel"
        />
        <div class="id-actions">
          <button class="emd-btn" @click="cancel">{{ t("c.cancel") }}</button>
          <button class="emd-btn emd-btn-accent" @click="confirm">{{ t("c.confirm") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { t } from "../../i18n";

const props = withDefaults(
  defineProps<{ title: string; placeholder?: string; initial?: string }>(),
  { placeholder: "", initial: "" },
);
const emit = defineEmits<{ confirm: [value: string]; cancel: [] }>();

const value = ref(props.initial);
const inputRef = ref<HTMLInputElement>();

onMounted(() => {
  inputRef.value?.focus();
  inputRef.value?.select();
});

function confirm() {
  if (value.value.trim()) emit("confirm", value.value.trim());
}
function cancel() {
  emit("cancel");
}
</script>

<style scoped>
.input-dialog {
  width: 400px;
  padding: 16px;
  gap: 12px;
}
.id-title {
  font-size: var(--font-ui-medium);
  margin-bottom: 12px;
}
.id-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
