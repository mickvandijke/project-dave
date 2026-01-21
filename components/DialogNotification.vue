<script lang="ts" setup>
import { useDialogNotification } from "~/stores/notifications";

// Support both store-based and prop-based usage for backwards compatibility
const props = defineProps<{
  visible?: boolean;
  notifyType?: 'info' | 'warning';
  title?: string;
  details?: string;
  canCancel?: boolean;
}>();

const emit = defineEmits(["close-notify"]);

// Get notification store
const notification = useDialogNotification();

// Computed values that prefer props over store (for backwards compatibility)
const isVisible = computed(() => props.visible ?? notification.isVisible);
const currentTitle = computed(() => props.title ?? notification.title);
const currentDetails = computed(() => props.details ?? notification.details);
const currentCanCancel = computed(() => props.canCancel ?? notification.canCancel);

const handleCancelNotify = () => {
  // If using props, emit event
  if (props.visible !== undefined) {
    emit("close-notify");
  }
  // If using store, call store's cancel
  notification.cancel();
};
</script>

<template>
  <Dialog
      :visible="isVisible"
      pt:root:class="!border-0 !bg-transparent"
      pt:mask:class="backdrop-blur-sm"
      position="topright"
  >
    <template #container="{ closeCallback }">
      <div
          class="flex flex-col px-8 py-8 gap-6 rounded-2xl bg-autonomi-blue-600 dark:bg-autonomi-blue-700 gap-y-1 "
      >
        <div class="flex justify-start">
          <img src="~/assets/img/autonomi-logo-text-white.svg" alt="Autonomi" class="h-6"/>
        </div>

        <div class="mt-6 text-white font-semibold flex items-center gap-2">
          <i class="pi pi-spin pi-spinner text-autonomi-red-300"/>{{ currentTitle }}
        </div>
        <div
            class="flex items-center justify-center gap-4 text-autonomi-text-primary mt-2"
        >
          <span>{{ currentDetails }}</span>
        </div>

        <div v-if="currentCanCancel" class="mt-4">
          <CommonButton
              variant="secondary"
              size="small"
              @click="handleCancelNotify"
              class="flex"
          >
            <span aria-label="Cancel">Cancel</span>
          </CommonButton>
        </div>
      </div>
    </template>
  </Dialog>
</template>
