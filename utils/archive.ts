/**
 * Utility functions for working with archives and file access structures
 */

import type { IArchive, IFailedArchive } from '~/types/folder';

/**
 * Archive access type - can be private or public
 */
export type ArchiveAccess = { Private: string } | { Public: string };

/**
 * File access type - can be private (bytes) or public (string address)
 */
export type FileAccess = { Private: number[] | Uint8Array } | { Public: string };

/**
 * Generate a unique key for tracking an archive by address and privacy type
 * @param address - The archive/file address
 * @param isPrivate - Whether the archive is private
 * @returns A unique key string
 */
export function getAddressKey(address: string, isPrivate: boolean): string {
  return isPrivate ? `private:${address}` : `public:${address}`;
}

/**
 * Parse an address key back to its components
 * @param key - The address key (e.g., "private:abc123")
 * @returns Object with address and isPrivate flag
 */
export function parseAddressKey(key: string): { address: string; isPrivate: boolean } {
  if (key.startsWith('private:')) {
    return { address: key.substring(8), isPrivate: true };
  } else if (key.startsWith('public:')) {
    return { address: key.substring(7), isPrivate: false };
  }
  // Default to public if no prefix
  return { address: key, isPrivate: false };
}

/**
 * Extract the address string from an archive_access or file_access structure
 * @param access - The access structure (Private or Public variant)
 * @returns The address string, or null if not found
 */
export function extractAddressFromAccess(access: ArchiveAccess | FileAccess | null | undefined): string | null {
  if (!access) return null;

  if ('Public' in access) {
    return access.Public as string;
  } else if ('Private' in access) {
    // For archive access, Private is a string
    // For file access, Private is bytes - but we still want the address if it's a string
    const privateValue = access.Private;
    if (typeof privateValue === 'string') {
      return privateValue;
    }
    // If it's bytes, we can't extract an address
    return null;
  }
  return null;
}

/**
 * Check if an access structure is for a private archive/file
 * @param access - The access structure
 * @returns True if private, false if public
 */
export function isPrivateAccess(access: ArchiveAccess | FileAccess | null | undefined): boolean {
  if (!access) return false;
  return 'Private' in access;
}

/**
 * Get the address from an archive structure
 * @param archive - The archive object
 * @returns The address string
 */
export function getArchiveAddress(archive: IArchive | IFailedArchive): string {
  if ('Private' in archive.archive_access) {
    return archive.archive_access.Private;
  }
  return archive.archive_access.Public;
}

/**
 * Check if an archive is private
 * @param archive - The archive object
 * @returns True if the archive is private
 */
export function isArchivePrivate(archive: IArchive | IFailedArchive): boolean {
  return 'Private' in archive.archive_access;
}

/**
 * Create an archive access structure
 * @param address - The archive address
 * @param isPrivate - Whether the archive is private
 * @returns The archive access structure
 */
export function createArchiveAccess(address: string, isPrivate: boolean): ArchiveAccess {
  return isPrivate ? { Private: address } : { Public: address };
}

/**
 * Compare two archives by their address to check if they're the same
 * @param a - First archive
 * @param b - Second archive
 * @returns True if they have the same address
 */
export function isSameArchive(a: IArchive | IFailedArchive, b: IArchive | IFailedArchive): boolean {
  return getArchiveAddress(a) === getArchiveAddress(b);
}

/**
 * Generate a unique temp code for tracking load operations
 * @param prefix - Optional prefix for the code (e.g., 'vault', 'local')
 * @returns A unique string code
 */
export function generateTempCode(prefix: string = 'load'): string {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}
