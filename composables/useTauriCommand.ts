/**
 * Composable for invoking Tauri commands with consistent error handling
 * and loading state management
 */
import { invoke } from '@tauri-apps/api/core';
import { formatErrorMessage } from './useNotifications';

export interface CommandState<T> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
}

export interface CommandOptions {
  /** Show error toast on failure */
  showErrorToast?: boolean;
  /** Custom error message prefix */
  errorPrefix?: string;
  /** Callback on success */
  onSuccess?: (data: unknown) => void;
  /** Callback on error */
  onError?: (error: Error) => void;
}

/**
 * Composable for managing Tauri command invocations
 * @returns Command invocation utilities
 */
export function useTauriCommand() {
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  /**
   * Invoke a Tauri command with error handling
   * @param command - The command name
   * @param args - Optional command arguments
   * @param options - Optional configuration
   * @returns Promise with the command result
   */
  const execute = async <T>(
    command: string,
    args?: Record<string, unknown>,
    options?: CommandOptions
  ): Promise<T | null> => {
    isLoading.value = true;
    error.value = null;

    try {
      const result = await invoke<T>(command, args);
      options?.onSuccess?.(result);
      return result;
    } catch (err) {
      const errorMessage = formatErrorMessage(err, `Failed to execute ${command}`);
      error.value = options?.errorPrefix
        ? `${options.errorPrefix}: ${errorMessage}`
        : errorMessage;

      if (err instanceof Error) {
        options?.onError?.(err);
      } else {
        options?.onError?.(new Error(errorMessage));
      }

      console.error(`[TauriCommand] ${command} failed:`, err);
      return null;
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Execute a command and throw on error (for use with try/catch)
   * @param command - The command name
   * @param args - Optional command arguments
   * @returns Promise with the command result
   * @throws Error if the command fails
   */
  const executeOrThrow = async <T>(
    command: string,
    args?: Record<string, unknown>
  ): Promise<T> => {
    isLoading.value = true;
    error.value = null;

    try {
      const result = await invoke<T>(command, args);
      return result;
    } catch (err) {
      const errorMessage = formatErrorMessage(err, `Failed to execute ${command}`);
      error.value = errorMessage;
      console.error(`[TauriCommand] ${command} failed:`, err);
      throw err instanceof Error ? err : new Error(errorMessage);
    } finally {
      isLoading.value = false;
    }
  };

  return {
    isLoading: readonly(isLoading),
    error: readonly(error),
    execute,
    executeOrThrow
  };
}

/**
 * Create a typed command executor for a specific Tauri command
 * @param command - The command name
 * @returns A function that executes the command with type safety
 */
export function createCommand<TArgs extends Record<string, unknown>, TResult>(
  command: string
) {
  return async (args?: TArgs): Promise<TResult> => {
    return invoke<TResult>(command, args);
  };
}

/**
 * Common Tauri commands used throughout the app
 */
export const tauriCommands = {
  appData: createCommand<Record<string, never>, AppData>('app_data'),
  appDataStore: createCommand<{ appData: AppData }, void>('app_data_store'),
  getUniqueDownloadPath: createCommand<
    { downloadsPath: string; filename: string },
    string
  >('get_unique_download_path'),
  showItemInFileManager: createCommand<{ path: string }, void>('show_item_in_file_manager'),
  downloadPublicFile: createCommand<{ addr: string; toDest: string }, void>('download_public_file'),
  downloadPrivateFile: createCommand<
    { dataMapChunk: number[]; toDest: string },
    void
  >('download_private_file')
};

/**
 * App data structure from Tauri
 */
export interface AppData {
  download_path?: string;
  use_paymaster?: boolean;
  [key: string]: unknown;
}
