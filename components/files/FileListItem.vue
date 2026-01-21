<script lang="ts" setup>
/**
 * FileListItem - Individual file/folder row in list view
 */

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
  file: FileItem;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  /** Emitted when the row is clicked (for navigation) */
  'row-click': [file: FileItem];
  /** Emitted when file name is clicked (for file actions) */
  'name-click': [file: FileItem];
  /** Emitted when menu button is clicked */
  'menu-click': [event: MouseEvent, file: FileItem];
}>();

const handleRowClick = () => {
  if (!props.file.is_loading_archive) {
    emit('row-click', props.file);
  }
};

const handleNameClick = (event: MouseEvent) => {
  event.stopPropagation();
  emit('name-click', props.file);
};

const handleMenuClick = (event: MouseEvent) => {
  event.stopPropagation();
  emit('menu-click', event, props.file);
};

// Helper to determine file icon based on extension
const getFileIcon = (fileName: string): string => {
  if (/\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(fileName)) {
    return 'pi pi-image';
  }
  if (/\.(pdf)$/i.test(fileName)) {
    return 'pi pi-file-pdf';
  }
  if (/\.(zip)$/i.test(fileName)) {
    return 'pi pi-box';
  }
  return 'pi pi-file';
};

const isFile = computed(() => !!props.file.path);
const isFolder = computed(() => !props.file.path && !props.file.isArchive && !props.file.is_failed_archive && !props.file.is_loading_archive);
const showMenu = computed(() => (props.file.path || props.file.isArchive || props.file.is_failed_archive) && !props.file.is_loading_archive);
</script>

<template>
  <div
    class="grid grid-cols-subgrid col-span-12 h-11 items-center odd:bg-autonomi-gray-100 dark:odd:bg-autonomi-blue-700 dark:text-autonomi-text-primary-dark"
    @click="handleRowClick"
    :class="{
      'cursor-pointer': (!file.path || file.is_failed_archive) && !file.is_loading_archive,
      'opacity-75': file.is_loading || file.is_loading_archive,
      'opacity-75 bg-red-100 dark:bg-red-900/20 hover:bg-red-200': file.load_error || file.is_failed_archive,
      'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive,
      'hover:bg-white dark:hover:bg-white/10': !(file.load_error || file.is_failed_archive || file.is_loading_archive)
    }"
  >
    <!-- Folder/File Name -->
    <div class="col-span-11 pl-[6rem] flex items-center">
      <!-- Failed archive -->
      <template v-if="file.is_failed_archive">
        <i class="pi pi-exclamation-triangle mr-4 text-red-500" />
        <i class="pi pi-box mr-2 text-red-500" />
        <span class="text-ellipsis overflow-hidden whitespace-nowrap text-red-600 dark:text-red-400">
          {{ file.name }}
        </span>
      </template>

      <!-- Loading archive -->
      <template v-else-if="file.is_loading_archive">
        <i class="pi pi-spinner pi-spin mr-4 text-blue-500" />
        <i class="pi pi-box mr-2 text-blue-500" />
        <span class="text-ellipsis overflow-hidden whitespace-nowrap text-blue-600 dark:text-blue-400">
          {{ file.name }} (loading...)
        </span>
      </template>

      <!-- File -->
      <template v-else-if="isFile">
        <i :class="getFileIcon(file.name)" class="mr-4" />
        <span
          class="text-ellipsis overflow-hidden whitespace-nowrap cursor-pointer"
          @click="handleNameClick"
        >
          {{ file.name }}
        </span>
        <!-- Loading indicators -->
        <i v-if="file.is_loading" class="pi pi-spinner pi-spin ml-2 text-sm text-blue-500" />
        <i
          v-else-if="file.load_error"
          class="pi pi-exclamation-triangle ml-2 text-sm text-red-500"
          v-tooltip.top="'Failed to load file data'"
        />
      </template>

      <!-- Folder or Archive -->
      <template v-else>
        <i :class="file.isArchive ? 'pi pi-box mr-4 text-amber-600 dark:text-amber-400' : 'pi pi-folder mr-4'" />
        <span class="text-ellipsis overflow-hidden whitespace-nowrap">{{ file.name }}</span>
      </template>
    </div>

    <!-- Menu -->
    <div class="col-span-1">
      <i
        v-if="showMenu"
        class="pi pi-ellipsis-v cursor-pointer hover:text-autonomi-gray-600 dark:hover:text-white"
        @click="handleMenuClick"
      />
    </div>
  </div>
</template>
