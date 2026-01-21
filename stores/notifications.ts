/**
 * Store for managing dialog notifications
 * Replaces the emit('show-notify') / emit('hide-notify') pattern
 */
import { defineStore } from 'pinia';

export type NotifyType = 'info' | 'warning' | 'error' | 'success';

export interface DialogNotification {
  visible: boolean;
  title: string;
  details: string;
  notifyType: NotifyType;
  canCancel: boolean;
  onCancel?: () => void;
}

const DEFAULT_NOTIFICATION: DialogNotification = {
  visible: false,
  title: '',
  details: '',
  notifyType: 'info',
  canCancel: false
};

export const useNotificationStore = defineStore('notifications', () => {
  // State for the dialog notification (the persistent one, not toast)
  const dialogNotification = ref<DialogNotification>({ ...DEFAULT_NOTIFICATION });

  /**
   * Show a dialog notification
   * @param options - Notification options
   */
  const showNotification = (options: {
    title: string;
    details: string;
    notifyType?: NotifyType;
    canCancel?: boolean;
    onCancel?: () => void;
  }): void => {
    dialogNotification.value = {
      visible: true,
      title: options.title,
      details: options.details,
      notifyType: options.notifyType ?? 'info',
      canCancel: options.canCancel ?? false,
      onCancel: options.onCancel
    };
  };

  /**
   * Hide the current dialog notification
   */
  const hideNotification = (): void => {
    dialogNotification.value = { ...DEFAULT_NOTIFICATION };
  };

  /**
   * Handle cancel action (calls onCancel callback if provided)
   */
  const handleCancel = (): void => {
    const onCancel = dialogNotification.value.onCancel;
    hideNotification();
    onCancel?.();
  };

  // Computed properties for template binding
  const isVisible = computed(() => dialogNotification.value.visible);
  const title = computed(() => dialogNotification.value.title);
  const details = computed(() => dialogNotification.value.details);
  const notifyType = computed(() => dialogNotification.value.notifyType);
  const canCancel = computed(() => dialogNotification.value.canCancel);

  return {
    // State
    dialogNotification: readonly(dialogNotification),

    // Computed
    isVisible,
    title,
    details,
    notifyType,
    canCancel,

    // Actions
    showNotification,
    hideNotification,
    handleCancel
  };
});

/**
 * Composable wrapper for easier use in components
 */
export function useDialogNotification() {
  const store = useNotificationStore();

  return {
    show: store.showNotification,
    hide: store.hideNotification,
    cancel: store.handleCancel,
    isVisible: store.isVisible,
    title: store.title,
    details: store.details,
    notifyType: store.notifyType,
    canCancel: store.canCancel
  };
}
