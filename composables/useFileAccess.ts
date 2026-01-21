/**
 * Composable for working with file access data
 * Handles extracting addresses, secret keys, and determining file types
 */
import { bytesToHex } from '~/utils/hex';
import type { FileAccessType } from '~/types/folder';

export interface FileAccessInfo {
  address: string;
  isPrivate: boolean;
  isArchive: boolean;
  rawBytes?: number[];
}

/**
 * File object structure from vault or local files
 */
export interface FileObject {
  file_access?: {
    Private?: number[] | string;
    Public?: string;
  };
  access_data?: {
    Private?: number[] | string;
    Public?: string;
  };
  archive_access?: {
    Private?: string;
    Public?: string;
  };
  type?: FileAccessType;
  name?: string;
  path?: string;
  isArchive?: boolean;
}

/**
 * Extract the data address from a file object (for public files)
 * @param file - The file object
 * @returns The data address string, or empty string if not available
 */
export function extractDataAddress(file: FileObject | null | undefined): string {
  if (!file) return '';

  // Check file_access.Public first
  if (file.file_access?.Public) {
    return file.file_access.Public;
  }

  // Check access_data.Public as alternative
  if (file.access_data?.Public) {
    return file.access_data.Public;
  }

  // Check archive_access.Public for archives
  if (file.archive_access?.Public) {
    return file.archive_access.Public;
  }

  return '';
}

/**
 * Extract the secret key / data map from a file object (for private files)
 * @param file - The file object
 * @returns The secret key as a hex string (without 0x prefix), or empty string if not available
 */
export function extractSecretKey(file: FileObject | null | undefined): string {
  if (!file) return '';

  let secretKey = '';

  // Check file_access.Private first
  if (file.file_access?.Private) {
    const privateData = file.file_access.Private;

    // Convert to hex string if it's an array
    if (Array.isArray(privateData)) {
      secretKey = bytesToHex(privateData);
    } else if (typeof privateData === 'string') {
      secretKey = privateData.startsWith('0x') ? privateData.slice(2) : privateData;
    }
  }
  // Check access_data.Private as alternative
  else if (file.access_data?.Private) {
    const privateData = file.access_data.Private;

    if (Array.isArray(privateData)) {
      secretKey = bytesToHex(privateData);
    } else if (typeof privateData === 'string') {
      secretKey = privateData.startsWith('0x') ? privateData.slice(2) : privateData;
    }
  }
  // Check archive_access.Private for archives
  else if (file.archive_access?.Private) {
    secretKey = file.archive_access.Private;
  }

  return secretKey;
}

/**
 * Determine if a file is public
 * @param file - The file object
 * @returns True if the file is public
 */
export function isPublicFile(file: FileObject | null | undefined): boolean {
  if (!file) return false;

  // Check type field
  if (file.type === 'public_file') return true;

  // Check file_access
  if (file.file_access?.Public) return true;

  // Check access_data
  if (file.access_data?.Public) return true;

  // Check archive_access
  if (file.archive_access?.Public) return true;

  return false;
}

/**
 * Determine if a file is private
 * @param file - The file object
 * @returns True if the file is private
 */
export function isPrivateFile(file: FileObject | null | undefined): boolean {
  if (!file) return false;

  // Check type field
  if (file.type === 'private_file') return true;

  // Check file_access
  if (file.file_access?.Private) return true;

  // Check access_data
  if (file.access_data?.Private) return true;

  // Check archive_access
  if (file.archive_access?.Private) return true;

  return false;
}

/**
 * Get comprehensive file access info
 * @param file - The file object
 * @returns FileAccessInfo with address and type information
 */
export function getFileAccessInfo(file: FileObject | null | undefined): FileAccessInfo | null {
  if (!file) return null;

  // Check if it's an archive
  if (file.archive_access || file.isArchive) {
    const isPrivate = !!file.archive_access?.Private;
    const address = file.archive_access?.Private || file.archive_access?.Public || '';

    return {
      address,
      isPrivate,
      isArchive: true
    };
  }

  // Regular file
  if (file.file_access?.Public || file.access_data?.Public) {
    return {
      address: file.file_access?.Public || file.access_data?.Public || '',
      isPrivate: false,
      isArchive: false
    };
  }

  if (file.file_access?.Private || file.access_data?.Private) {
    const privateData = file.file_access?.Private || file.access_data?.Private;
    let address = '';
    let rawBytes: number[] | undefined;

    if (Array.isArray(privateData)) {
      address = '0x' + bytesToHex(privateData);
      rawBytes = privateData;
    } else if (typeof privateData === 'string') {
      address = privateData.startsWith('0x') ? privateData : `0x${privateData}`;
    }

    return {
      address,
      isPrivate: true,
      isArchive: false,
      rawBytes
    };
  }

  return null;
}

/**
 * Get the file access object for Tauri commands
 * @param file - The file object
 * @returns The file_access object suitable for Tauri commands
 */
export function getFileAccess(
  file: FileObject | null | undefined
): { Private: number[] } | { Public: string } | null {
  if (!file) return null;

  // Check file_access first
  if (file.file_access) {
    if (file.file_access.Public) {
      return { Public: file.file_access.Public };
    }
    if (file.file_access.Private) {
      if (Array.isArray(file.file_access.Private)) {
        return { Private: file.file_access.Private };
      }
      // If it's a hex string, convert to bytes
      const hex = file.file_access.Private.startsWith('0x')
        ? file.file_access.Private.slice(2)
        : file.file_access.Private;
      const bytes = Array.from(
        hex.match(/.{1,2}/g) || [],
        (byte: string) => parseInt(byte, 16)
      );
      return { Private: bytes };
    }
  }

  // Check access_data as alternative
  if (file.access_data) {
    if (file.access_data.Public) {
      return { Public: file.access_data.Public };
    }
    if (file.access_data.Private) {
      if (Array.isArray(file.access_data.Private)) {
        return { Private: file.access_data.Private };
      }
      const hex = file.access_data.Private.startsWith('0x')
        ? file.access_data.Private.slice(2)
        : file.access_data.Private;
      const bytes = Array.from(
        hex.match(/.{1,2}/g) || [],
        (byte: string) => parseInt(byte, 16)
      );
      return { Private: bytes };
    }
  }

  return null;
}

/**
 * Composable for working with file access in components
 */
export function useFileAccess() {
  return {
    extractDataAddress,
    extractSecretKey,
    isPublicFile,
    isPrivateFile,
    getFileAccessInfo,
    getFileAccess
  };
}
