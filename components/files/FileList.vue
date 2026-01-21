<script lang="ts" setup>
/**
 * FileList - List view for files and folders
 */
import FileListItem from './FileListItem.vue';

interface FileItem {
  name: string;
  path?: string;
  isArchive?: boolean;
  is_failed_archive?: boolean;
  is_loading_archive?: boolean;
  is_loading?: boolean;
  load_error?: boolean;
  archive?: unknown;
}

interface Props {
  /** Array of files/folders to display */
  files: FileItem[];
  /** Whether the list is loading */
  isLoading?: boolean;
  /** Whether to show the load vault button */
  showLoadButton?: boolean;
  /** Label for the load button */
  loadButtonLabel?: string;
}

const props = withDefaults(defineProps<Props>(), {
  isLoading: false,
  showLoadButton: false,
  loadButtonLabel: 'Load Vault'
});

const emit = defineEmits<{
  /** Emitted when a file/folder row is clicked */
  'item-click': [file: FileItem];
  /** Emitted when a file name is clicked */
  'name-click': [file: FileItem];
  /** Emitted when a menu button is clicked */
  'menu-click': [event: MouseEvent, file: FileItem];
  /** Emitted when load button is clicked */
  'load-click': [];
}>();

const handleRowClick = (file: FileItem) => {
  emit('item-click', file);
};

const handleNameClick = (file: FileItem) => {
  emit('name-click', file);
};

const handleMenuClick = (event: MouseEvent, file: FileItem) => {
  emit('menu-click', event, file);
};

const handleLoadClick = () => {
  emit('load-click');
};
</script>

<template>
  <div
    class="mt-6 overflow-y-auto overscroll-none"
    style="height: calc(100vh - 280px);"
  >
    <div class="grid grid-cols-12 font-semibold mb-10">
      <!-- Header -->
      <div class="col-span-11 pl-[6rem] text-autonomi-red-300">
        Name
      </div>
      <div class="col-span-1 text-autonomi-red-300">
        <i class="pi pi-user" />
      </div>

      <!-- Spacer -->
      <div class="col-span-12 h-10" />

      <!-- Files Rows -->
      <template v-if="files.length">
        <FileListItem
          v-for="file in files"
          :key="file.path || file.name"
          :file="file"
          @row-click="handleRowClick"
          @name-click="handleNameClick"
          @menu-click="handleMenuClick"
        />
      </template>

      <!-- Empty state -->
      <template v-else>
        <div class="col-span-12 p-8 text-center text-gray-500">
          <div v-if="isLoading">
            <i class="pi pi-spinner pi-spin mr-4" />Loading vault...
          </div>
          <div v-else-if="showLoadButton" class="flex justify-center">
            <Button
              :label="loadButtonLabel"
              icon="pi pi-globe"
              @click="handleLoadClick"
              class="mt-4"
            />
          </div>
          <div v-else>No files found.</div>
        </div>
      </template>
    </div>
  </div>
</template>
