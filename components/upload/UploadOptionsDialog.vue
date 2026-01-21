<script lang="ts" setup>
/**
 * UploadOptionsDialog - Modal for configuring upload options
 * Allows users to set privacy, vault storage, and payment options before uploading
 */

export interface UploadOptions {
  files: File[];
  isFolder: boolean;
  isPrivate: boolean;
  addToVault: boolean;
  useCachedReceipts: boolean;
}

interface Props {
  visible: boolean;
  options: UploadOptions;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  'confirm': [options: UploadOptions];
  'cancel': [];
}>();

// Local copy of options for editing
const localOptions = ref<UploadOptions>({ ...props.options });

// Watch for prop changes to sync local state
watch(() => props.options, (newOptions) => {
  localOptions.value = { ...newOptions };
}, { deep: true });

// Computed for v-model binding on dialog
const dialogVisible = computed({
  get: () => props.visible,
  set: (value) => emit('update:visible', value)
});

const handleConfirm = () => {
  emit('confirm', { ...localOptions.value });
};

const handleCancel = () => {
  emit('cancel');
};

// Display helpers
const fileTypeLabel = computed(() => {
  if (localOptions.value.isFolder) return 'Folder';
  return localOptions.value.files.length === 1 ? 'File' : 'Files';
});

const fileNameDisplay = computed(() => {
  if (localOptions.value.isFolder) {
    return localOptions.value.files[0]?.name || 'Unknown folder';
  }
  if (localOptions.value.files.length === 1) {
    return localOptions.value.files[0]?.name || 'Unknown file';
  }
  return `${localOptions.value.files.length} files`;
});
</script>

<template>
  <Dialog
    v-model:visible="dialogVisible"
    modal
    header="Upload Options"
    :style="{ width: '450px' }"
    :closable="true"
  >
    <div class="flex flex-col gap-6 p-1">
      <!-- File Info -->
      <div class="bg-gray-50 dark:bg-autonomi-blue-600 rounded-lg p-4">
        <div class="flex items-center gap-3">
          <i
            :class="localOptions.isFolder ? 'pi pi-folder' : 'pi pi-file'"
            class="text-autonomi-blue-500"
          />
          <div>
            <div class="font-semibold text-sm">{{ fileTypeLabel }}</div>
            <div class="text-sm text-gray-600 dark:text-autonomi-secondary-dark">
              {{ fileNameDisplay }}
            </div>
          </div>
        </div>
      </div>

      <!-- Privacy Options -->
      <div class="space-y-3">
        <div class="flex items-center gap-2">
          <label class="text-sm font-semibold">Privacy</label>
          <i
            class="pi pi-info-circle text-sm text-gray-500 cursor-help"
            v-tooltip="{
              value: 'Private files require a data map to access them. Public files can be accessed by anyone who has the data address.',
              showDelay: 300,
              hideDelay: 300
            }"
          />
        </div>
        <div class="space-y-3">
          <div class="flex items-center">
            <RadioButton
              v-model="localOptions.isPrivate"
              inputId="private"
              name="privacy"
              :value="true"
            />
            <label for="private" class="ml-2 flex items-center gap-2 cursor-pointer">
              <i class="pi pi-lock text-autonomi-blue-500" />
              <div class="flex-1">
                <div class="font-medium">Private</div>
              </div>
              <i
                class="pi pi-info-circle text-xs text-gray-400 cursor-help"
                v-tooltip="{
                  value: 'Files will be uploaded to the network, but they will only be accessible with the data map file.\n\nThis data map file will be stored locally on your device and will be viewable in your local vault.',
                  showDelay: 300,
                  hideDelay: 300,
                  autoHide: false
                }"
              />
            </label>
          </div>
          <div class="flex items-center">
            <RadioButton
              v-model="localOptions.isPrivate"
              inputId="public"
              name="privacy"
              :value="false"
            />
            <label for="public" class="ml-2 flex items-center gap-2 cursor-pointer">
              <i class="pi pi-globe text-green-500" />
              <div class="flex-1">
                <div class="font-medium">Public</div>
              </div>
              <i
                class="pi pi-info-circle text-xs text-gray-400 cursor-help"
                v-tooltip="{
                  value: 'Files will be uploaded to the network and will be accessible to anyone with the data address.',
                  showDelay: 300,
                  hideDelay: 300,
                  autoHide: false
                }"
              />
            </label>
          </div>
        </div>
      </div>

      <!-- Vault Options -->
      <div class="space-y-3">
        <div class="flex items-center gap-2">
          <label class="text-sm font-semibold">Storage Options</label>
        </div>
        <div class="flex items-center">
          <Checkbox
            v-model="localOptions.addToVault"
            inputId="vault"
            :binary="true"
          />
          <label for="vault" class="ml-2 flex items-center gap-2 cursor-pointer">
            <i class="pi pi-database text-autonomi-blue-500" />
            <div class="flex-1">
              <div class="font-medium">Add to Personal Vault</div>
            </div>
            <i
              class="pi pi-info-circle text-xs text-gray-400 cursor-help"
              v-tooltip="{
                value: 'Store a reference to your files in your personal vault for easy access from anywhere. When unchecked, files are stored only on the network and referenced in your local vault.',
                showDelay: 300,
                hideDelay: 300,
                autoHide: false
              }"
            />
          </label>
        </div>
      </div>

      <!-- Cached Receipts Option -->
      <div class="space-y-3">
        <div class="flex items-center gap-2">
          <label class="text-sm font-semibold">Payment Options</label>
        </div>
        <div class="flex items-center">
          <Checkbox
            v-model="localOptions.useCachedReceipts"
            inputId="use-cached"
            :binary="true"
          />
          <label for="use-cached" class="ml-2 flex items-center gap-2 cursor-pointer">
            <i class="pi pi-clock text-autonomi-blue-500" />
            <div class="flex-1">
              <div class="font-medium">Use Cached Receipts</div>
              <div class="text-xs text-gray-500">If available</div>
            </div>
            <i
              class="pi pi-info-circle text-xs text-gray-400 cursor-help"
              v-tooltip="{
                value: 'Use previously cached payment receipts when available. If partial coverage, only pay for missing chunks. When unchecked, always requests fresh payment.',
                showDelay: 300,
                hideDelay: 300,
                autoHide: false
              }"
            />
          </label>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end gap-3">
        <Button
          label="Cancel"
          severity="secondary"
          @click="handleCancel"
          outlined
        />
        <Button
          label="Upload"
          @click="handleConfirm"
        />
      </div>
    </template>
  </Dialog>
</template>
