<script lang="ts" setup>
import {useWalletStore} from "~/stores/wallet";
import {storeToRefs} from "pinia";
import {useNotifications} from "~/composables/useNotifications";

// Login
const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(["close-disconnect-wallet"]);
const { showSuccess, showError } = useNotifications();
const walletStore = useWalletStore();
const {pendingDisconnectWallet, wallet} = storeToRefs(walletStore);

const {visible} = props;

const handleCancelDisconnect = () => {
  emit("close-disconnect-wallet");
};

const handleDisconnectWallet = async () => {
  try {
    const response = await walletStore.disconnectWallet();

    if (response.success) {
      showSuccess("Success", "Mobile wallet disconnected successfully");
      navigateTo("/");
    } else {
      throw new Error("Failed to disconnect mobile wallet");
    }
  } catch (error) {
    showError("Error", error as Error);
  } finally {
    emit("close-disconnect-wallet");
  }
};
</script>

<template>
  <Dialog
      :visible="props.visible"
      pt:root:class="!border-0 !bg-transparent"
      pt:mask:class="backdrop-blur-sm"
      position="topright"
  >
    <template #container="{ closeCallback }">
      <div
          class="flex flex-col px-8 py-8 gap-6 rounded-2xl bg-autonomi-blue-600 dark:bg-autonomi-blue-700"
      >
        <div class="flex justify-start">
          <img src="~/assets/img/autonomi-logo-text-white.svg" alt="Autonomi" class="h-6"/>
        </div>

        <div
            v-if="pendingDisconnectWallet || !wallet.isConnected"
            class="flex items-center justify-center gap-4 text-autonomi-text-primary"
        >
          <i class="pi pi-spin pi-spinner text-autonomi-red-300"/>
          <span>Disconnecting...</span>
        </div>
        <div v-else class="flex items-center justify-center flex-wrap gap-4">
          <CommonButton
              variant="tertiary"
              @click="handleCancelDisconnect"
              size="large"
          >
            Cancel
          </CommonButton>
          <CommonButton
              variant="primary"
              size="large"
              @click="handleDisconnectWallet"
          >
            <i class="pi pi-times-circle"/>Disconnect Your Mobile Wallet
          </CommonButton>
        </div>
      </div>
    </template>
  </Dialog>
</template>
