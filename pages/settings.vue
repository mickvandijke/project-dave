<script lang="ts" setup>
import { onMounted } from 'vue';
import { useSettings } from '~/composables/useSettings';
import { useNotifications } from '~/composables/useNotifications';

const settings = useSettings();
const { showSuccess, showError } = useNotifications();

// Destructure for template convenience
const { downloadPath, usePaymaster, appVersion, isLoading, isSaving } = settings;

// Load current settings
const loadSettings = async () => {
  try {
    await settings.loadSettings();
  } catch (error) {
    showError('Error', 'Failed to load settings');
  }
};

// Choose directory and auto-save
const chooseDirectory = async () => {
  try {
    const selected = await settings.chooseDownloadDirectory();
    if (selected) {
      showSuccess('Success', 'Download directory updated');
    }
  } catch (error) {
    showError('Error', 'Failed to save download directory');
  }
};

// Open logs folder
const openLogsFolder = async () => {
  try {
    await settings.openLogsFolder();
  } catch (error) {
    showError('Error', 'Failed to open logs folder');
  }
};

// Auto-save paymaster settings when toggled
const onPaymasterToggle = async () => {
  try {
    await settings.setUsePaymaster(!usePaymaster.value);
    showSuccess(
      'Success',
      usePaymaster.value
        ? 'Paymaster enabled - gas-free transactions active'
        : 'Paymaster disabled - standard payments active'
    );
  } catch (error) {
    showError('Error', 'Failed to update paymaster settings');
  }
};

onMounted(async () => {
  loadSettings();
  try {
    await settings.loadAppVersion();
  } catch (error) {
    console.error('Failed to get app version:', error);
  }
});
</script>

<template>
  <div class="px-[66px] lg:px-[110px] pt-[70px] pb-10">
    <h1 class="text-3xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-2">
      Settings
    </h1>
    <p class="text-autonomi-text-primary mb-8">
      Configure your preferences and download settings.
    </p>

    <div v-if="isLoading" class="flex items-center justify-center py-20">
      <ProgressSpinner/>
    </div>

    <div v-else class="bg-white dark:bg-white/10 rounded-lg p-6 shadow-sm">
      <div class="space-y-6">
        <!-- Download Directory Section -->
        <div>
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Download Directory
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            Choose where your downloaded files will be saved.
          </p>

          <div class="flex gap-3 items-center">
            <div class="flex-1">
              <InputText
                  :model-value="downloadPath"
                  :disabled="true"
                  placeholder="No directory selected"
                  class="w-full"
              />
            </div>
            <CommonButton
                variant="secondary"
                size="medium"
                @click="chooseDirectory"
                :disabled="isSaving"
                :loading="isSaving"
            >
              Browse...
            </CommonButton>
          </div>

          <div v-if="downloadPath" class="mt-2">
            <p class="text-sm text-autonomi-text-secondary dark:text-autonomi-text-secondary-dark">
              Current: {{ downloadPath }}
            </p>
          </div>
        </div>

        <!-- Paymaster Settings Section -->
        <div class="border-t border-white/10 pt-6">
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Paymaster Settings
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            Enable paymaster to pay for transactions using only ANT tokens without needing ETH for gas fees.
          </p>

          <div class="flex items-center gap-3">
            <Checkbox
              :model-value="usePaymaster"
              inputId="usePaymaster"
              binary
              @change="onPaymasterToggle"
              :disabled="isSaving"
            />
            <label for="usePaymaster" class="text-sm text-autonomi-text-primary cursor-pointer">
              Enable Paymaster (Gas-free transactions)
            </label>
          </div>
        </div>

        <!-- Logs Directory Section -->
        <div class="border-t border-white/10 pt-6">
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Application Logs
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            View application logs for troubleshooting and debugging.
          </p>

          <CommonButton
              variant="secondary"
              size="medium"
              @click="openLogsFolder"
          >
            Open Logs Folder
          </CommonButton>
        </div>
      </div>
    </div>

    <!-- Version info at the bottom -->
    <div class="mt-8 text-center">
      <p class="text-sm text-autonomi-text-secondary dark:text-autonomi-text-secondary-dark">
        Dave version {{ appVersion }}
      </p>
    </div>
  </div>
</template>
