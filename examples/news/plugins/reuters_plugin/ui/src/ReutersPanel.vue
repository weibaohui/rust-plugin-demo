<script setup lang="ts">
import { ref, onMounted } from 'vue';

interface ReutersPanelProps {
  pluginId?: string;
}

const props = withDefaults(defineProps<ReutersPanelProps>(), {
  pluginId: 'reuters_plugin',
});

const dateline = ref('LONDON');
const note = ref('');
const flash = ref<{ type: 'success' | 'error'; text: string } | null>(null);

const DATELINE_OPTIONS = [
  { label: 'London', value: 'LONDON' },
  { label: 'New York', value: 'NEW YORK' },
  { label: 'Tokyo', value: 'TOKYO' },
  { label: 'Hong Kong', value: 'HONG KONG' },
];

interface DemoRow {
  key: string;
  city: string;
  region: string;
}

const demoData: DemoRow[] = [
  { key: '1', city: 'London', region: 'Europe' },
  { key: '2', city: 'New York', region: 'Americas' },
  { key: '3', city: 'Tokyo', region: 'Asia' },
  { key: '4', city: 'Hong Kong', region: 'Asia' },
];

const demoColumns = [
  { title: '城市', key: 'city' },
  { title: '地区', key: 'region' },
];

onMounted(() => {
  const raw = localStorage.getItem(`plugin-settings-${props.pluginId}`);
  if (raw) {
    try {
      const s = JSON.parse(raw) as { dateline?: string; note?: string };
      if (typeof s.dateline === 'string') dateline.value = s.dateline;
      if (typeof s.note === 'string') note.value = s.note;
    } catch {
      /* ignore malformed settings */
    }
  }
});

function handleSave(): void {
  try {
    localStorage.setItem(
      `plugin-settings-${props.pluginId}`,
      JSON.stringify({ dateline: dateline.value, note: note.value }),
    );
    flash.value = { type: 'success', text: '设置已保存' };
  } catch {
    flash.value = { type: 'error', text: '保存失败,请重试' };
  }
  setTimeout(() => {
    flash.value = null;
  }, 2000);
}
</script>

<template>
  <n-card title="📰 路透社控制面板" style="max-width: 720px;">
    <n-space vertical :size="24">
      <n-form label-placement="top">
        <n-form-item label="电头 (Dateline)">
          <n-select v-model:value="dateline" :options="DATELINE_OPTIONS" />
        </n-form-item>
        <n-form-item label="备注">
          <n-input v-model:value="note" placeholder="输入备注信息" clearable />
        </n-form-item>
        <n-form-item label="当前插件 ID">
          <n-input :value="pluginId" disabled />
        </n-form-item>
        <n-button type="primary" @click="handleSave">💾 保存设置</n-button>
      </n-form>
      <n-card title="电头演示数据" size="small" embedded>
        <n-data-table
          :columns="demoColumns"
          :data="demoData"
          :pagination="false"
          size="small"
        />
      </n-card>
      <n-alert v-if="flash" :type="flash.type" :title="flash.text" />
    </n-space>
  </n-card>
</template>
