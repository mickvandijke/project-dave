import { useWalletStore, type MerklePaymentOrder, type PoolCommitment, type MerklePaymentResult } from "./wallet";
import { invoke } from "@tauri-apps/api/core";

export type PaymentOrder = {
  id: number;
  payments: [string, string, string][];  // Array of tuples, not array containing single tuple
};

// Merkle payment order from backend
export type BackendMerklePaymentOrder = {
  upload_id: string;
  depth: number;
  pool_commitments: any[];  // Serialized pool commitments from backend
  merkle_payment_timestamp: number;
  estimated_cost: string;
  total_files: number;
  total_size: number;
};

export enum ProcessingState {
  PENDING,
  PROCESSING,
  COMPLETED,
  CANCELLED,
}

export type PendingPayment = {
  order: PaymentOrder;
  expires: number;
  processing: ProcessingState;
};

export type PendingMerklePayment = {
  uploadId: string;
  depth: number;
  poolCommitments: PoolCommitment[];
  merklePaymentTimestamp: bigint;
  estimatedCost: bigint;
  totalFiles: number;
  totalSize: bigint;
  expires: number;
  processing: ProcessingState;
};

export const usePaymentStore = defineStore("payments", () => {
  const walletStore = useWalletStore();

  // State
  const IDLE_PAYMENT_EXPIRATION_TIME_SECS = ref<number>(600);
  const currentPayment = ref<any>(null);
  const pendingPayments: Ref<Map<number, PendingPayment>> = ref(new Map());
  const pendingMerklePayments: Ref<Map<string, PendingMerklePayment>> = ref(new Map());
  const showPayments = ref(false);
  const signPaymentPending = ref(false);

  const pendingPaymentsCount = computed(
    () =>
      Array.from(pendingPayments.value.values()).filter(
        (payment) =>
          payment.processing === ProcessingState.PENDING ||
          payment.processing === ProcessingState.PROCESSING
      ).length
  );

  const sortedPendingPayments = computed(() => {
    return Array.from(pendingPayments.value.values())
      .filter((payment) => payment.expires > Date.now())
      .sort((a, b) => a.expires - b.expires);
  });

  // Methods

  const addPendingPayment = (orderId: number, orderData: any) => {
    console.log(">>> ADDING PAYMENT", orderData);
    
    // Create a proper PendingPayment structure
    const pendingPayment: PendingPayment = {
      order: {
        id: orderData.id,
        payments: orderData.payments
      },
      expires: Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS.value,
      processing: ProcessingState.PENDING
    };
    
    pendingPayments.value.set(orderId, pendingPayment);

    // Update current payment
    currentPayment.value = pendingPayment;
    console.log(">>> Current payment set:", currentPayment.value);
  };

  const resetExpirationTime = (orderId: number) => {
    const payment = pendingPayments.value.get(orderId);

    if (payment) {
      payment.expires =
        Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS.value; // Reset expiration to 120 seconds from now
      pendingPayments.value.set(orderId, payment);
    } else {
      console.error(`Order with ID ${orderId} not found in pending payments.`);
    }
  };

  const setProcessingState = (orderId: number, state: ProcessingState) => {
    const payment = pendingPayments.value.get(orderId);

    if (payment) {
      payment.processing = state;
      pendingPayments.value.set(orderId, payment);
    } else {
      console.error(`Order with ID ${orderId} not found in pending payments.`);
    }
  };

  const getProcessingState = (orderId: number): ProcessingState | undefined => {
    const payment = pendingPayments.value.get(orderId);
    return payment ? payment.processing : undefined;
  };

  const setCurrentPayment = (payment: any) => {
    // TODO: Use correc ttype
    currentPayment.value = payment;
  };

  const setPaymentView = (view: string) => {
    // TODO: Implement payment view state if needed
    console.log("Setting payment view to:", view);
  };

  //   const calculateRemainingTime = (expires: number) =>
  //     Math.max(0, Math.floor((expires - Date.now()) / 1000));

  const calculateRemainingTime = (expires: number) => {
    const totalSeconds = Math.max(0, Math.floor((expires - Date.now()) / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(
      2,
      "0"
    )}:${String(seconds).padStart(2, "0")}`;
  };

  const calculateTotalAmount = (
    payments: [string, string, string][]
  ): bigint => {
    return payments.reduce((total, payment) => {
      const amountHex = payment[2];
      const amount = BigInt(amountHex);
      return total + amount;
    }, BigInt(0));
  };

  const pay = async (order: PaymentOrder) => {
    let processingState = getProcessingState(order.id);

    if (
      processingState === ProcessingState.PROCESSING ||
      processingState === ProcessingState.COMPLETED
    ) {
      return;
    }

    try {
      console.log(">>> Attempting to pay for order", order);
      // Set payment pending - flag to show notification
      signPaymentPending.value = true;

      await invoke("send_payment_order_message", {
        id: order.id,
        message: "KeepAlive",
      });

      setProcessingState(order.id, ProcessingState.PROCESSING);
      resetExpirationTime(order.id);

      await walletStore.payForQuotes(order.payments);
      setProcessingState(order.id, ProcessingState.COMPLETED);
      await invoke("send_payment_order_message", {
        id: order.id,
        message: "Completed",
      });
      signPaymentPending.value = false;
      console.log(">>> Payment complete");
    } catch (err) {
      console.error(">>> Error paying for quote", err);

      // Set sign pending false - flag to remove notification
      signPaymentPending.value = false;

      setProcessingState(order.id, ProcessingState.CANCELLED);
      await invoke("send_payment_order_message", {
        id: order.id,
        message: "Cancelled",
      });
    }
  };

  const cancel = async (orderId: number) => {
    setProcessingState(orderId, ProcessingState.CANCELLED);
    await invoke("send_payment_order_message", {
      id: orderId,
      message: "Cancelled",
    });
  };

  // Merkle Payment Methods

  /**
   * Converts backend merkle payment order to frontend format
   */
  // Helper to convert byte array to hex string
  const bytesToHex = (bytes: number[]): `0x${string}` => {
    return `0x${bytes.map(b => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`;
  };

  // Helper to safely convert to BigInt, handling null/undefined (for Option<u64> from Rust)
  const safeBigInt = (value: number | string | null | undefined): bigint => {
    if (value === null || value === undefined) {
      return BigInt(0);
    }
    return BigInt(value);
  };

  /**
   * CRITICAL: Data type conversion matching the autonomi Rust crate.
   * The backend uses one enum scheme, but the smart contract expects a different mapping.
   * This conversion is applied in evmlib/src/contract/mod.rs before contract calls.
   *
   * Backend -> Contract mapping:
   *   0 (Chunk)      -> 2
   *   1 (GraphEntry) -> 0
   *   2 (Pointer)    -> 3
   *   3 (Scratchpad) -> 1
   *   other          -> 4 (Does not exist)
   */
  const dataTypeConversion = (dataType: number): number => {
    switch (dataType) {
      case 0: return 2; // Chunk
      case 1: return 0; // GraphEntry
      case 2: return 3; // Pointer
      case 3: return 1; // Scratchpad
      default: return 4; // Does not exist
    }
  };

  const convertBackendMerkleOrder = (backendOrder: BackendMerklePaymentOrder): PendingMerklePayment => {
    console.log("[convertBackendMerkleOrder] === CONVERSION DEBUG ===");
    console.log("[convertBackendMerkleOrder] Backend order depth:", backendOrder.depth);
    console.log("[convertBackendMerkleOrder] Backend pool_commitments count:", backendOrder.pool_commitments.length);
    console.log("[convertBackendMerkleOrder] Expected pool count (2^ceil(depth/2)):", 1 << Math.ceil(backendOrder.depth / 2));

    // Convert pool commitments from backend format
    const poolCommitments: PoolCommitment[] = backendOrder.pool_commitments.map((pc: any, poolIndex: number) => {
      console.log(`[convertBackendMerkleOrder] Pool ${poolIndex}:`);
      console.log(`  - pool_hash bytes length: ${pc.pool_hash?.length}`);
      console.log(`  - candidates count: ${pc.candidates?.length} (expected: 16)`);

      // Validate pool_hash is exactly 32 bytes
      if (pc.pool_hash?.length !== 32) {
        console.error(`[convertBackendMerkleOrder] ERROR: pool_hash has ${pc.pool_hash?.length} bytes, expected 32`);
      }

      // Validate exactly 16 candidates
      if (pc.candidates?.length !== 16) {
        console.error(`[convertBackendMerkleOrder] ERROR: Pool ${poolIndex} has ${pc.candidates?.length} candidates, expected 16`);
      }

      const poolHash = bytesToHex(pc.pool_hash);
      console.log(`  - poolHash (hex): ${poolHash}`);

      return {
        // pool_hash is a byte array, convert to hex string
        poolHash,
        candidates: pc.candidates.map((c: any, candIndex: number) => {
          // Apply data type conversion to match the smart contract's expected format
          const convertedDataType = dataTypeConversion(c.metrics.data_type);

          // Log first candidate of each pool for debugging
          if (candIndex === 0) {
            console.log(`  - First candidate rewards_address: ${c.rewards_address}`);
            console.log(`  - First candidate data_type: ${c.metrics.data_type} -> converted to: ${convertedDataType}`);
            console.log(`  - First candidate close_records_stored: ${c.metrics.close_records_stored}`);
            console.log(`  - First candidate records_per_type (raw):`, c.metrics.records_per_type);
            console.log(`  - First candidate records_per_type (converted):`, (c.metrics.records_per_type || []).map((r: [number, number]) => [dataTypeConversion(r[0]), r[1]]));
          }

          // IMPORTANT: Only send the 3 fields the contract expects
          // (matching autonomi crate's encoding - see evmlib/src/contract/merkle_payment_vault/interface.rs)
          return {
            rewardsAddress: c.rewards_address,
            metrics: {
              // CRITICAL: Apply dataTypeConversion to match Rust crate behavior
              dataType: convertedDataType,
              closeRecordsStored: safeBigInt(c.metrics.close_records_stored),
              // records_per_type is array of tuples [data_type, records]
              // CRITICAL: Also convert dataType in each record
              recordsPerType: (c.metrics.records_per_type || []).map((r: [number, number]) => ({
                dataType: dataTypeConversion(r[0]),
                records: BigInt(r[1])
              })),
            }
          };
        })
      };
    });

    console.log("[convertBackendMerkleOrder] Conversion complete");

    return {
      uploadId: backendOrder.upload_id,
      depth: backendOrder.depth,
      poolCommitments,
      merklePaymentTimestamp: BigInt(backendOrder.merkle_payment_timestamp),
      estimatedCost: BigInt(backendOrder.estimated_cost),
      totalFiles: backendOrder.total_files,
      totalSize: BigInt(backendOrder.total_size),
      expires: Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS.value,
      processing: ProcessingState.PENDING
    };
  };

  /**
   * Adds a pending merkle payment from a backend event
   */
  const addPendingMerklePayment = (backendOrder: BackendMerklePaymentOrder) => {
    console.log(">>> ADDING MERKLE PAYMENT", backendOrder);

    const pendingMerklePayment = convertBackendMerkleOrder(backendOrder);
    pendingMerklePayments.value.set(backendOrder.upload_id, pendingMerklePayment);

    console.log(">>> Merkle payment added:", pendingMerklePayment);
  };

  /**
   * Sets the processing state for a merkle payment
   */
  const setMerkleProcessingState = (uploadId: string, state: ProcessingState) => {
    const payment = pendingMerklePayments.value.get(uploadId);

    if (payment) {
      payment.processing = state;
      pendingMerklePayments.value.set(uploadId, payment);
    } else {
      console.error(`Merkle payment with upload ID ${uploadId} not found.`);
    }
  };

  /**
   * Gets the processing state for a merkle payment
   */
  const getMerkleProcessingState = (uploadId: string): ProcessingState | undefined => {
    const payment = pendingMerklePayments.value.get(uploadId);
    return payment ? payment.processing : undefined;
  };

  /**
   * Pays for a merkle tree payment
   */
  const payMerkle = async (uploadId: string): Promise<MerklePaymentResult | null> => {
    const payment = pendingMerklePayments.value.get(uploadId);

    if (!payment) {
      console.error(`Merkle payment with upload ID ${uploadId} not found.`);
      return null;
    }

    const processingState = getMerkleProcessingState(uploadId);
    if (
      processingState === ProcessingState.PROCESSING ||
      processingState === ProcessingState.COMPLETED
    ) {
      console.log("Merkle payment already processing or completed");
      return null;
    }

    try {
      console.log(">>> Attempting to pay for merkle payment", payment);
      signPaymentPending.value = true;

      setMerkleProcessingState(uploadId, ProcessingState.PROCESSING);

      // Execute the merkle tree payment
      const result = await walletStore.payForMerkleTree(
        payment.depth,
        payment.poolCommitments,
        payment.merklePaymentTimestamp
      );

      setMerkleProcessingState(uploadId, ProcessingState.COMPLETED);

      // Confirm the upload with the backend
      await invoke("confirm_merkle_upload_payment", {
        uploadId,
        merkleResult: {
          winner_pool_hash: result.winnerPoolHash,
          amount_paid: result.amountPaid
        }
      });

      signPaymentPending.value = false;
      console.log(">>> Merkle payment complete", result);

      return result;
    } catch (err) {
      console.error(">>> Error paying for merkle payment", err);
      signPaymentPending.value = false;
      setMerkleProcessingState(uploadId, ProcessingState.CANCELLED);
      throw err;
    }
  };

  /**
   * Cancels a merkle payment
   */
  const cancelMerkle = (uploadId: string) => {
    setMerkleProcessingState(uploadId, ProcessingState.CANCELLED);
    pendingMerklePayments.value.delete(uploadId);
  };

  // Return values
  return {
    currentPayment,
    IDLE_PAYMENT_EXPIRATION_TIME_SECS,
    pendingPayments,
    pendingMerklePayments,
    pendingPaymentsCount,
    signPaymentPending,
    sortedPendingPayments,
    addPendingPayment,
    addPendingMerklePayment,
    calculateRemainingTime,
    calculateTotalAmount,
    getProcessingState,
    getMerkleProcessingState,
    resetExpirationTime,
    setCurrentPayment,
    setPaymentView,
    pay,
    payMerkle,
    cancel,
    cancelMerkle,
  };
});
