/**
 * Utility functions for formatting values in the UI
 */

/**
 * Format an amount in ATTO (10^-18 ANT) to a human-readable ANT string
 * @param attoAmount - The amount in ATTO as a string (to handle large numbers)
 * @param maxDecimals - Maximum number of decimal places to show (default: 12)
 * @returns Formatted ANT amount string
 */
export function formatANT(attoAmount: string, maxDecimals: number = 12): string {
  try {
    // Convert string to BigInt
    const atto = BigInt(attoAmount);

    // 1 ANT = 10^18 ATTO
    const ATTO_PER_ANT = BigInt(1_000_000_000_000_000_000n);

    // Divide to get ANT amount
    const antAmount = atto / ATTO_PER_ANT;
    const remainder = atto % ATTO_PER_ANT;

    // Format with decimal places if there's a remainder
    if (remainder === BigInt(0)) {
      return antAmount.toString();
    } else {
      // Calculate decimal places (up to 18 decimals)
      const decimalStr = remainder.toString().padStart(18, '0');
      // Remove trailing zeros
      const trimmed = decimalStr.replace(/0+$/, '');
      if (trimmed.length === 0) {
        return antAmount.toString();
      }
      // Show up to maxDecimals for better precision
      const displayDecimals = trimmed.substring(0, maxDecimals);
      return `${antAmount}.${displayDecimals}`;
    }
  } catch (error) {
    console.error('Error formatting ANT amount:', error);
    return '0';
  }
}

/**
 * Format a byte count to a human-readable string
 * @param bytes - Number of bytes
 * @returns Formatted string (e.g., "1.5 MB")
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

/**
 * Format a bigint ANT amount using viem's formatUnits
 * This is useful when you already have a bigint value
 * @param attoAmount - The amount in ATTO as bigint
 * @param maxDecimals - Maximum number of decimal places to show (default: 6)
 * @returns Formatted ANT amount string
 */
export function formatANTFromBigInt(attoAmount: bigint, maxDecimals: number = 6): string {
  try {
    // Manual conversion since we don't want to import viem here for the basic version
    const ATTO_PER_ANT = BigInt(1_000_000_000_000_000_000n);
    const antAmount = attoAmount / ATTO_PER_ANT;
    const remainder = attoAmount % ATTO_PER_ANT;

    if (remainder === BigInt(0)) {
      return antAmount.toString();
    }

    const decimalStr = remainder.toString().padStart(18, '0');
    const trimmed = decimalStr.replace(/0+$/, '');

    if (trimmed.length === 0) {
      return antAmount.toString();
    }

    const displayDecimals = trimmed.substring(0, maxDecimals);
    return `${antAmount}.${displayDecimals}`;
  } catch (error) {
    console.error('Error formatting ANT amount:', error);
    return '0';
  }
}

/**
 * Format a bigint ANT amount with rounding up for display
 * Useful when showing minimum required amounts
 * @param attoAmount - The amount in ATTO as bigint
 * @param maxDecimals - Maximum number of decimal places (default: 6)
 * @returns Formatted ANT amount string, rounded up at the last displayed decimal
 */
export function formatANTRoundedUp(attoAmount: bigint, maxDecimals: number = 6): string {
  try {
    const ATTO_PER_ANT = BigInt(1_000_000_000_000_000_000n);
    const antAmount = attoAmount / ATTO_PER_ANT;
    const remainder = attoAmount % ATTO_PER_ANT;

    if (remainder === BigInt(0)) {
      return antAmount.toString();
    }

    const decimalStr = remainder.toString().padStart(18, '0');
    let trimmed = decimalStr.replace(/0+$/, '');

    if (trimmed.length === 0) {
      return antAmount.toString();
    }

    // Truncate and round up the last digit
    if (trimmed.length > maxDecimals) {
      trimmed = trimmed.substring(0, maxDecimals);
      const lastDigit = parseInt(trimmed[trimmed.length - 1]);
      const roundedUp = lastDigit === 9 ? '9' : (lastDigit + 1).toString();
      trimmed = trimmed.substring(0, trimmed.length - 1) + roundedUp;
    } else {
      const lastDigit = parseInt(trimmed[trimmed.length - 1]);
      const roundedUp = lastDigit === 9 ? '9' : (lastDigit + 1).toString();
      trimmed = trimmed.substring(0, trimmed.length - 1) + roundedUp;
    }

    // Remove trailing zeros after rounding
    trimmed = trimmed.replace(/0+$/, '');

    if (trimmed.length === 0) {
      return antAmount.toString();
    }

    return `${antAmount}.${trimmed}`;
  } catch (error) {
    console.error('Error formatting ANT amount with rounding:', error);
    return '0';
  }
}

/**
 * Format a Unix timestamp to a human-readable date string
 * @param timestamp - Unix timestamp in seconds
 * @returns Formatted date string
 */
export function formatTimestamp(timestamp: number): string {
  if (!timestamp) return 'Unknown';
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
}
