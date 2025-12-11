import {WagmiPlugin} from "@wagmi/vue";
import {QueryClient, VueQueryPlugin} from "@tanstack/vue-query";
import {wagmiConfig} from "~/config";

// Nuxt 3 app aliases
import {defineNuxtPlugin} from "#imports";

export default defineNuxtPlugin((nuxt) => {
    const queryClient = new QueryClient();

    nuxt.vueApp.use(WagmiPlugin, {config: wagmiConfig});
    nuxt.vueApp.use(VueQueryPlugin, {queryClient});
})
