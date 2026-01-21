<script lang="ts" setup>
/**
 * FileBreadcrumbs - Reusable breadcrumb navigation component for file views
 */
import type { IFolder } from '~/types/folder';

interface Props {
  /** Array of folder crumbs to display */
  crumbs: IFolder[];
  /** Label for the root item (e.g., "Vault" or "Local Vault") */
  rootLabel: string;
  /** The root directory object */
  rootDirectory: IFolder | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  /** Emitted when a breadcrumb is clicked */
  'crumb-click': [folder: IFolder | null];
}>();

const handleRootClick = () => {
  emit('crumb-click', props.rootDirectory);
};

const handleCrumbClick = (crumb: IFolder) => {
  emit('crumb-click', crumb);
};
</script>

<template>
  <div
    v-if="crumbs?.length > 0"
    class="mx-[6rem] flex gap-4 items-center text-sm font-semibold flex-wrap my-4"
  >
    <!-- Root label -->
    <div
      class="cursor-pointer transition-all duration-300 text-autonomi-text-secondary dark:text-autonomi-text-primary-dark"
      @click="handleRootClick"
    >
      {{ rootLabel }}
    </div>
    <i class="text-xs pi pi-arrow-right text-autonomi-text-primary/70 dark:text-autonomi-text-primary-dark/70" />

    <!-- Crumbs -->
    <template v-for="(crumb, index) in crumbs" :key="index">
      <div
        :class="`cursor-pointer transition-all duration-300 ${
          index === crumbs.length - 1
            ? 'text-autonomi-text-secondary dark:text-autonomi-text-secondary-dark'
            : 'text-autonomi-text-primary/70 dark:text-autonomi-text-primary-dark/70'
        }`"
        @click="handleCrumbClick(crumb)"
      >
        {{ crumb.name }}
      </div>
      <i
        v-if="index !== crumbs.length - 1"
        class="text-xs pi pi-arrow-right text-autonomi-text-primary/70 dark:text-autonomi-text-primary-dark/70"
      />
    </template>
  </div>
</template>
