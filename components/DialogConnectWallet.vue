<script lang="ts" setup>
import {useToast} from "primevue/usetoast";
import {useWalletStore} from "~/stores/wallet";
import {storeToRefs} from "pinia";
// Login
const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(["close-login"]);
const toast = useToast();
const walletStore = useWalletStore();
const {pendingConnectWallet, wallet, callbackConnectWallet} = storeToRefs(walletStore);

const {visible} = props;

const handleCloseLogIn = () => {
  emit("close-login");
};

const handleLogIn = async () => {
  try {
    console.log(">>> Running login with WalletConnect");

    const response = await walletStore.connectWallet();

    console.log(">>> Response", response);

    if (response.success) {
      if (callbackConnectWallet.value) {
        callbackConnectWallet.value();
      }
      emit("close-login");
    } else {
      // If wrong chain, disconnect the wallet first
      if (response.wrongChain) {
        await walletStore.disconnectWallet();
      }

      // Show error toast with the actual error message
      toast.add({
        severity: "error",
        summary: response.wrongChain ? "Wrong Network" : "Connection Failed",
        detail: response.message || "Failed to connect wallet",
        life: 8000,
      });
      emit("close-login");
    }
  } catch (error: any) {
    console.error("Unexpected error during wallet connection:", error);
    toast.add({
      severity: "error",
      summary: "Error",
      detail: error.message || "Failed to connect wallet",
      life: 5000,
    });
    emit("close-login");
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
            v-if="pendingConnectWallet || wallet.isConnected"
            class="flex items-center justify-center gap-4 text-autonomi-text-primary"
        >
          <i class="pi pi-spin pi-spinner"/>
          <span>Connecting...</span>
        </div>
        <div v-else class="flex flex-col gap-4">
          <CommonButton variant="primary" size="large" @click="handleLogIn">
            <i class="pi pi-qrcode"/> Connect Wallet
          </CommonButton>
          <CommonButton
              variant="tertiary"
              size="large"
              @click="handleCloseLogIn"
          >
            Cancel
          </CommonButton>
        </div>
      </div>
    </template>
  </Dialog>
</template>
