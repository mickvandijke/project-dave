/**
 * Composable for toast notifications
 * Provides a simplified API for showing toast messages
 */
import { useToast } from 'primevue/usetoast';

export type ToastSeverity = 'success' | 'info' | 'warn' | 'error';

export interface ToastOptions {
  summary: string;
  detail?: string;
  life?: number;
  closable?: boolean;
  group?: string;
}

/**
 * Default toast display duration in milliseconds
 */
const DEFAULT_LIFE = 3000;
const ERROR_LIFE = 5000;

/**
 * Composable for managing toast notifications
 * @returns Toast notification helpers
 */
export function useNotifications() {
  const toast = useToast();

  /**
   * Show a toast notification
   * @param severity - The type of notification
   * @param options - Toast options
   */
  const show = (severity: ToastSeverity, options: ToastOptions) => {
    const life = options.life ?? (severity === 'error' ? ERROR_LIFE : DEFAULT_LIFE);

    toast.add({
      severity,
      summary: options.summary,
      detail: options.detail,
      life,
      closable: options.closable ?? true,
      group: options.group
    });
  };

  /**
   * Show a success notification
   * @param summary - Brief message
   * @param detail - Optional detailed message
   * @param life - Optional display duration in ms
   */
  const showSuccess = (summary: string, detail?: string, life?: number) => {
    show('success', { summary, detail, life });
  };

  /**
   * Show an error notification
   * @param summary - Brief message
   * @param detail - Optional detailed message (or Error object)
   * @param life - Optional display duration in ms (default: 5000)
   */
  const showError = (summary: string, detail?: string | Error, life?: number) => {
    const detailText = detail instanceof Error ? detail.message : detail;
    show('error', { summary, detail: detailText, life: life ?? ERROR_LIFE });
  };

  /**
   * Show an info notification
   * @param summary - Brief message
   * @param detail - Optional detailed message
   * @param life - Optional display duration in ms
   */
  const showInfo = (summary: string, detail?: string, life?: number) => {
    show('info', { summary, detail, life });
  };

  /**
   * Show a warning notification
   * @param summary - Brief message
   * @param detail - Optional detailed message
   * @param life - Optional display duration in ms
   */
  const showWarn = (summary: string, detail?: string, life?: number) => {
    show('warn', { summary, detail, life });
  };

  /**
   * Remove all toast notifications
   * @param group - Optional group to clear
   */
  const clearAll = (group?: string) => {
    toast.removeAllGroups();
  };

  return {
    show,
    showSuccess,
    showError,
    showInfo,
    showWarn,
    clearAll,
    // Expose raw toast for advanced usage
    toast
  };
}

/**
 * Helper to format error messages for display
 * @param error - Error object or string
 * @param fallback - Fallback message if error is not informative
 * @returns User-friendly error message
 */
export function formatErrorMessage(error: unknown, fallback: string = 'An unexpected error occurred'): string {
  if (error instanceof Error) {
    return error.message || fallback;
  }
  if (typeof error === 'string') {
    return error || fallback;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message) || fallback;
  }
  return fallback;
}
