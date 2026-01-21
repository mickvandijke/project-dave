<script lang="ts" setup>
import {usePaymentStore, ProcessingState} from "~/stores/payments";
import { formatANT } from "~/utils/formatting";
import { useCountdown } from "~/composables/useCountdown";

const emit = defineEmits(["payment-pay", "payment-cancel"]);
const paymentStore = usePaymentStore();
const {payment} = defineProps<{
  payment: any;
}>();

// Use countdown composable for timer
const countdown = useCountdown();
const remainingTime = countdown.remainingTime;

onMounted(() => {
  countdown.start(payment.expires);
});
</script>

<template>
  <div
      class="flex flex-col gap-1 text-xs p-4 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50 transition-all duration-300"
  >
    <div class="flex justify-between items-center">
      <div class="flex flex-col gap-1">
        <div class="font-semibold text-sm">
          Payment ID: {{ payment.order.id }}
        </div>
        <div>
          <div>Total amount: {{ formatANT(paymentStore.calculateTotalAmount(payment.order.payments)) }} ANT</div>
          <div class="text-xs text-gray-500">{{ paymentStore.calculateTotalAmount(payment.order.payments) }} ATTO</div>
        </div>
        <div
            v-if="
            paymentStore.getProcessingState(payment.order.id) ===
            ProcessingState.PENDING
          "
        >
          Expires in: {{ remainingTime }}
        </div>
      </div>
      <div
          v-if="
          payment.processing === ProcessingState.PENDING ||
          payment.processing === ProcessingState.PROCESSING
        "
          v-tooltip="'Pay before timer expires.'"
      >
        <i class="pi pi-spin pi-spinner-dotted"/>
      </div>
    </div>
    <div class="mt-4 flex items-center gap-6">
      <template v-if="payment.processing === ProcessingState.COMPLETED">
        <div class="flex gap-2 items-center font-semibold text-green-600">
          <i class="pi pi-check-circle"/>
          Payment Complete
        </div>
      </template>
      <template v-else-if="payment.processing === ProcessingState.CANCELLED">
        <div class="flex gap-2 items-center font-semibold text-red-600">
          <i class="pi pi-times text-red-600"/>
          Payment Cancelled
        </div>
      </template>
      <template v-else>
        <CommonButton
            label="Pay"
            icon="pi pi-wallet"
            class="flex gap-1"
            variant="secondary"
            @click="emit('payment-pay', payment)"
        >
          <i class="pi pi-wallet"/>
          Pay & Upload
        </CommonButton>
        <CommonButton
            label="Cancel"
            icon="pi pi-times"
            class="flex gap-1"
            variant="tertiary"
            @click="emit('payment-cancel', payment)"
        >
          <i class="pi pi-times"/>
          Cancel
        </CommonButton>
      </template>
    </div>
  </div>
</template>
