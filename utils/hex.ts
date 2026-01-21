/**
 * Utility functions for handling hex strings and byte arrays
 */

/**
 * Remove the 0x prefix from a hex string if present
 * @param hex - Hex string that may or may not have 0x prefix
 * @returns Hex string without 0x prefix
 */
export function cleanHex(hex: string): string {
  const trimmed = hex.trim();
  return trimmed.toLowerCase().startsWith("0x") ? trimmed.substring(2) : trimmed;
}

/**
 * Add 0x prefix to a hex string if not present
 * @param hex - Hex string that may or may not have 0x prefix
 * @returns Hex string with 0x prefix
 */
export function ensureHexPrefix(hex: string): string {
  const trimmed = hex.trim();
  return trimmed.toLowerCase().startsWith("0x") ? trimmed : `0x${trimmed}`;
}

/**
 * Check if a string is a valid hex string (with or without 0x prefix)
 * @param hex - String to validate
 * @returns True if the string is valid hex
 */
export function isValidHex(hex: string): boolean {
  if (!hex || typeof hex !== 'string') return false;
  const clean = cleanHex(hex);
  return /^[0-9a-fA-F]+$/i.test(clean);
}

/**
 * Check if a hex string represents a valid data address (64 hex chars = 32 bytes)
 * @param hex - Hex string to validate
 * @returns True if it's a valid 32-byte data address
 */
export function isValidDataAddress(hex: string): boolean {
  if (!isValidHex(hex)) return false;
  const clean = cleanHex(hex);
  return clean.length === 64;
}

/**
 * Check if a hex string represents a data map (longer than 64 hex chars)
 * @param hex - Hex string to validate
 * @returns True if it's a valid data map hex
 */
export function isValidDataMapHex(hex: string): boolean {
  if (!isValidHex(hex)) return false;
  const clean = cleanHex(hex);
  return clean.length > 64;
}

/**
 * Detect the type of hex input (address or datamap)
 * @param hex - Hex string to analyze
 * @returns 'address' | 'datamap' | null
 */
export function detectHexType(hex: string): 'address' | 'datamap' | null {
  if (!hex || typeof hex !== 'string') return null;

  const trimmed = hex.trim();
  if (!trimmed) return null;

  // Remove 0x prefix if present for validation
  const clean = cleanHex(trimmed);

  // Check if it's a valid hex string (case insensitive)
  if (!/^[0-9a-fA-F]+$/i.test(clean)) {
    return null;
  }

  // Data address is exactly 64 hex characters
  if (clean.length === 64) {
    return "address";
  }
  // Data map hex is typically longer than 64 characters
  else if (clean.length > 64) {
    return "datamap";
  }

  return null;
}

/**
 * Convert a hex string to a byte array
 * @param hex - Hex string (with or without 0x prefix)
 * @returns Array of bytes (numbers 0-255)
 */
export function hexToBytes(hex: string): number[] {
  const clean = cleanHex(hex);
  if (!isValidHex(hex)) {
    throw new Error('Invalid hex string');
  }
  return Array.from(
    clean.match(/.{1,2}/g) || [],
    byte => parseInt(byte, 16)
  );
}

/**
 * Convert a byte array to a hex string
 * @param bytes - Array of bytes (numbers 0-255)
 * @param prefix - Whether to include 0x prefix (default: false)
 * @returns Hex string
 */
export function bytesToHex(bytes: number[] | Uint8Array, prefix: boolean = false): string {
  const hex = Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
  return prefix ? `0x${hex}` : hex;
}

/**
 * Truncate a hex address for display (e.g., "0x1234...5678")
 * @param hex - Hex string to truncate
 * @param startChars - Number of characters to show at start (default: 6)
 * @param endChars - Number of characters to show at end (default: 4)
 * @returns Truncated hex string
 */
export function truncateHex(hex: string, startChars: number = 6, endChars: number = 4): string {
  const clean = cleanHex(hex);
  if (clean.length <= startChars + endChars) {
    return hex;
  }
  return `0x${clean.substring(0, startChars)}...${clean.substring(clean.length - endChars)}`;
}
