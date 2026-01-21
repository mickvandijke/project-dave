/**
 * Composable for countdown timer functionality
 * Used for payment expiration timers and other time-sensitive features
 */

/**
 * Format remaining seconds as HH:MM:SS
 * @param totalSeconds - Total seconds remaining
 * @returns Formatted time string
 */
export function formatRemainingTime(totalSeconds: number): string {
  const safeSeconds = Math.max(0, totalSeconds);
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const seconds = safeSeconds % 60;

  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

/**
 * Calculate remaining seconds from an expiration timestamp
 * @param expiresAt - Expiration timestamp in milliseconds
 * @returns Remaining seconds (0 if expired)
 */
export function calculateRemainingSeconds(expiresAt: number): number {
  return Math.max(0, Math.floor((expiresAt - Date.now()) / 1000));
}

/**
 * Calculate and format remaining time from an expiration timestamp
 * @param expiresAt - Expiration timestamp in milliseconds
 * @returns Formatted time string (HH:MM:SS)
 */
export function calculateRemainingTime(expiresAt: number): string {
  const totalSeconds = calculateRemainingSeconds(expiresAt);
  return formatRemainingTime(totalSeconds);
}

/**
 * Composable for managing a countdown timer
 * @returns Countdown timer utilities
 */
export function useCountdown() {
  const remainingTime = ref<string>("00:00:00");
  const isExpired = ref<boolean>(false);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  /**
   * Start the countdown timer
   * @param expiresAt - Expiration timestamp in milliseconds
   * @param onExpire - Optional callback when timer expires
   */
  const start = (expiresAt: number, onExpire?: () => void) => {
    // Stop any existing timer
    stop();

    // Initial update
    const updateTime = () => {
      const time = calculateRemainingTime(expiresAt);
      remainingTime.value = time;

      if (time === "00:00:00") {
        isExpired.value = true;
        stop();
        onExpire?.();
      }
    };

    updateTime();
    isExpired.value = false;

    // Start interval
    intervalId = setInterval(updateTime, 1000);
  };

  /**
   * Stop the countdown timer
   */
  const stop = () => {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  };

  /**
   * Reset the timer
   */
  const reset = () => {
    stop();
    remainingTime.value = "00:00:00";
    isExpired.value = false;
  };

  // Cleanup on unmount
  onUnmounted(() => {
    stop();
  });

  return {
    remainingTime: readonly(remainingTime),
    isExpired: readonly(isExpired),
    start,
    stop,
    reset
  };
}

/**
 * Composable for a simple countdown with a specific value
 * @param initialExpiresAt - Optional initial expiration timestamp
 * @returns Countdown utilities with auto-start capability
 */
export function usePaymentCountdown(initialExpiresAt?: number) {
  const countdown = useCountdown();

  // Watch for expiration changes
  if (initialExpiresAt) {
    countdown.start(initialExpiresAt);
  }

  return countdown;
}
