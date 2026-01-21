/**
 * Composable for tracking archive loading state
 * Used by both files.ts and localFiles.ts stores
 */
import { getAddressKey, parseAddressKey } from '~/utils/archive';
import type { ArchiveAccess } from '~/types/folder';

export interface LoadingArchive {
  name: string;
  archive_access: ArchiveAccess;
}

/**
 * Composable for managing archive loading state
 * @param logPrefix - Optional prefix for console logs (e.g., 'LOCAL:')
 * @returns Archive loading state management utilities
 */
export function useArchiveLoading(logPrefix: string = '') {
  // State - use reactive Set and Map for tracking
  const loadingArchiveAddresses = ref<Set<string>>(new Set());
  const loadingArchiveNames = ref<Map<string, string>>(new Map());

  const prefix = logPrefix ? `>>> ${logPrefix} ` : '>>> ';

  /**
   * Add an archive to the loading state
   * @param name - Archive name
   * @param address - Archive address
   * @param isPrivate - Whether the archive is private
   */
  const addLoadingArchive = (name: string, address: string, isPrivate: boolean): void => {
    const key = getAddressKey(address, isPrivate);
    console.log(`${prefix}Adding loading archive:`, name, address, isPrivate ? 'Private' : 'Public');
    loadingArchiveAddresses.value.add(key);
    loadingArchiveNames.value.set(key, name);
    console.log(`${prefix}Current loading addresses:`, Array.from(loadingArchiveAddresses.value));
  };

  /**
   * Remove an archive from the loading state
   * @param address - Archive address
   * @param isPrivate - Whether the archive is private
   */
  const removeLoadingArchive = (address: string, isPrivate: boolean): void => {
    const key = getAddressKey(address, isPrivate);
    console.log(`${prefix}Removing loading archive:`, address, isPrivate ? 'Private' : 'Public');
    loadingArchiveAddresses.value.delete(key);
    loadingArchiveNames.value.delete(key);
    console.log(`${prefix}Current loading addresses:`, Array.from(loadingArchiveAddresses.value));
  };

  /**
   * Clear all loading archives
   */
  const clearAllLoadingArchives = (): void => {
    console.log(`${prefix}Clearing all loading archives`);
    loadingArchiveAddresses.value.clear();
    loadingArchiveNames.value.clear();
  };

  /**
   * Check if an archive is currently loading
   * @param address - Archive address
   * @param isPrivate - Whether the archive is private
   * @returns True if the archive is loading
   */
  const isArchiveLoading = (address: string, isPrivate: boolean): boolean => {
    const key = getAddressKey(address, isPrivate);
    return loadingArchiveAddresses.value.has(key);
  };

  /**
   * Get the loading archive list in the format expected by components
   */
  const loadingArchives = computed<LoadingArchive[]>(() => {
    return Array.from(loadingArchiveAddresses.value).map((key) => {
      const { address, isPrivate } = parseAddressKey(key);
      return {
        name: loadingArchiveNames.value.get(key) || 'Unknown',
        archive_access: isPrivate ? { Private: address } : { Public: address }
      };
    });
  });

  /**
   * Get the count of loading archives
   */
  const loadingCount = computed<number>(() => {
    return loadingArchiveAddresses.value.size;
  });

  /**
   * Check if any archives are currently loading
   */
  const hasLoadingArchives = computed<boolean>(() => {
    return loadingArchiveAddresses.value.size > 0;
  });

  return {
    // State
    loadingArchiveAddresses: readonly(loadingArchiveAddresses),
    loadingArchiveNames: readonly(loadingArchiveNames),
    loadingArchives,
    loadingCount,
    hasLoadingArchives,

    // Actions
    addLoadingArchive,
    removeLoadingArchive,
    clearAllLoadingArchives,
    isArchiveLoading
  };
}

/**
 * Create a shared archive loading instance for use across stores
 */
export function createArchiveLoadingState(logPrefix: string = '') {
  return useArchiveLoading(logPrefix);
}
