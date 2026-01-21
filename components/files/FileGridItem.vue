<script lang="ts" setup>
/**
 * FileGridItem - Individual file/folder tile in grid view
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
  /** Emitted when the tile is clicked (for navigation) */
  'tile-click': [file: FileItem];
  /** Emitted when file name is clicked (for file actions) */
  'name-click': [file: FileItem];
  /** Emitted when menu button is clicked */
  'menu-click': [event: MouseEvent, file: FileItem];
}>();

const handleTileClick = () => {
  if (!props.file.is_loading_archive) {
    emit('tile-click', props.file);
  }
};

const handleNameClick = (event: MouseEvent) => {
  event.stopPropagation();
  if (props.file.path) {
    emit('name-click', props.file);
  }
};

const handleMenuClick = (event: MouseEvent) => {
  event.stopPropagation();
  emit('menu-click', event, props.file);
};

const showMenu = computed(() => (props.file.path || props.file.isArchive || props.file.is_failed_archive) && !props.file.is_loading_archive);
const displayName = computed(() => props.file.is_loading_archive ? `${props.file.name} (loading...)` : props.file.name);
</script>

<template>
  <div
    class="aspect-square w-full text-autonomi-text-primary dark:text-autonomi-text-secondary-dark hover:bg-white rounded-lg hover:text-autonomi-text-secondary dark:hover:bg-white/10 dark:hover:text-autonomi-text-primary-dark dark:border-autonomi-blue-800 transition-all duration-500 p-3 border flex flex-col"
    :class="{
      'cursor-pointer': !file.is_loading_archive,
      'cursor-default opacity-75': file.is_loading_archive,
      'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive
    }"
    @click="handleTileClick"
  >
    <!-- Menu button -->
    <template v-if="showMenu">
      <div class="self-end mb-2">
        <i
          class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
          @click="handleMenuClick"
        />
      </div>
    </template>
    <template v-else>
      <div class="self-end mb-2 h-4"></div>
    </template>

    <!-- Icon and name -->
    <div class="flex flex-col items-center justify-center flex-1 min-h-0">
      <div class="flex-shrink-0 mb-3">
        <i v-if="file.is_failed_archive" class="pi pi-exclamation-triangle text-3xl text-red-500" />
        <i v-else-if="file.is_loading_archive" class="pi pi-spinner pi-spin text-3xl text-blue-500" />
        <i v-else-if="file.path" class="pi pi-file text-3xl" />
        <i
          v-else
          :class="file.isArchive ? 'pi pi-box text-3xl text-amber-600 dark:text-amber-400' : 'pi pi-folder text-3xl'"
        />
      </div>

      <div class="w-full px-1 min-h-0">
        <span
          class="text-center text-xs block w-full cursor-pointer overflow-hidden text-ellipsis"
          style="display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; word-break: break-word;"
          :title="displayName"
          @click="handleNameClick"
        >
          {{ displayName }}
        </span>
      </div>
    </div>
  </div>
</template>
