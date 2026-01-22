/**
 * Composable for managing application settings
 * Provides reactive access to app settings with auto-persistence
 */
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppData } from './useTauriCommand';

export interface Settings {
  downloadPath: string;
  usePaymaster: boolean;
  useMerklePayments: boolean;
}

/**
 * Composable for managing application settings
 * @returns Settings management utilities
 */
export function useSettings() {
  // State
  const downloadPath = ref<string>('');
  const usePaymaster = ref<boolean>(false);
  const useMerklePayments = ref<boolean>(false);
  const appVersion = ref<string>('');
  const isLoading = ref<boolean>(false);
  const isSaving = ref<boolean>(false);
  const error = ref<string | null>(null);

  /**
   * Load all settings from the backend
   */
  const loadSettings = async (): Promise<void> => {
    try {
      isLoading.value = true;
      error.value = null;

      const appData = await invoke<AppData>('app_data');
      downloadPath.value = appData.download_path || '';
      usePaymaster.value = appData.use_paymaster ?? false;
      useMerklePayments.value = appData.use_merkle_payments ?? false;
    } catch (err) {
      console.error('Failed to load settings:', err);
      error.value = 'Failed to load settings';
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Load the app version
   */
  const loadAppVersion = async (): Promise<string> => {
    try {
      const version = await invoke<string>('get_app_version');
      appVersion.value = version;
      return version;
    } catch (err) {
      console.error('Failed to get app version:', err);
      return '';
    }
  };

  /**
   * Save a single setting to the backend
   * @param key - The setting key
   * @param value - The setting value
   */
  const saveSetting = async <K extends keyof AppData>(
    key: K,
    value: AppData[K]
  ): Promise<void> => {
    try {
      isSaving.value = true;
      error.value = null;

      const currentAppData = await invoke<AppData>('app_data');
      const updatedAppData = {
        ...currentAppData,
        [key]: value
      };

      await invoke('app_data_store', { appData: updatedAppData });
    } catch (err) {
      console.error(`Failed to save setting ${key}:`, err);
      error.value = `Failed to save ${key}`;
      throw err;
    } finally {
      isSaving.value = false;
    }
  };

  /**
   * Update the download path
   * @param path - New download path
   */
  const setDownloadPath = async (path: string): Promise<void> => {
    const previousValue = downloadPath.value;
    downloadPath.value = path;

    try {
      await saveSetting('download_path', path);
    } catch (err) {
      // Revert on error
      downloadPath.value = previousValue;
      throw err;
    }
  };

  /**
   * Open a directory picker and set the download path
   * @returns The selected path, or null if cancelled
   */
  const chooseDownloadDirectory = async (): Promise<string | null> => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Choose Download Directory'
      });

      if (selected && selected !== downloadPath.value) {
        await setDownloadPath(selected as string);
        return selected as string;
      }

      return null;
    } catch (err) {
      console.error('Failed to select directory:', err);
      throw err;
    }
  };

  /**
   * Update the paymaster setting
   * @param enabled - Whether to enable paymaster
   */
  const setUsePaymaster = async (enabled: boolean): Promise<void> => {
    const previousValue = usePaymaster.value;
    usePaymaster.value = enabled;

    try {
      await saveSetting('use_paymaster', enabled);
    } catch (err) {
      // Revert on error
      usePaymaster.value = previousValue;
      throw err;
    }
  };

  /**
   * Toggle the paymaster setting
   */
  const togglePaymaster = async (): Promise<void> => {
    await setUsePaymaster(!usePaymaster.value);
  };

  /**
   * Update the merkle payments setting
   * @param enabled - Whether to enable merkle payments
   */
  const setUseMerklePayments = async (enabled: boolean): Promise<void> => {
    const previousValue = useMerklePayments.value;
    useMerklePayments.value = enabled;

    try {
      await saveSetting('use_merkle_payments', enabled);
    } catch (err) {
      // Revert on error
      useMerklePayments.value = previousValue;
      throw err;
    }
  };

  /**
   * Toggle the merkle payments setting
   */
  const toggleMerklePayments = async (): Promise<void> => {
    await setUseMerklePayments(!useMerklePayments.value);
  };

  /**
   * Open the logs folder in the system file manager
   */
  const openLogsFolder = async (): Promise<void> => {
    try {
      const logsPath = await invoke<string>('get_logs_directory');
      await invoke('show_item_in_file_manager', { path: logsPath });
    } catch (err) {
      console.error('Failed to open logs folder:', err);
      throw err;
    }
  };

  /**
   * Get the current download path (loads if not already loaded)
   */
  const getDownloadPath = async (): Promise<string> => {
    if (!downloadPath.value) {
      await loadSettings();
    }
    return downloadPath.value;
  };

  /**
   * Check if paymaster is enabled (loads if not already loaded)
   */
  const isPaymasterEnabled = async (): Promise<boolean> => {
    if (!downloadPath.value && !usePaymaster.value) {
      await loadSettings();
    }
    return usePaymaster.value;
  };

  return {
    // State
    downloadPath: readonly(downloadPath),
    usePaymaster: readonly(usePaymaster),
    useMerklePayments: readonly(useMerklePayments),
    appVersion: readonly(appVersion),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),

    // Actions
    loadSettings,
    loadAppVersion,
    setDownloadPath,
    chooseDownloadDirectory,
    setUsePaymaster,
    togglePaymaster,
    setUseMerklePayments,
    toggleMerklePayments,
    openLogsFolder,
    getDownloadPath,
    isPaymasterEnabled
  };
}

/**
 * Singleton settings store for cross-component usage
 * Use this when you need to share settings state between components
 */
let globalSettings: ReturnType<typeof useSettings> | null = null;

export function useGlobalSettings() {
  if (!globalSettings) {
    globalSettings = useSettings();
  }
  return globalSettings;
}
