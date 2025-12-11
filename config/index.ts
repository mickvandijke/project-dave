import { http, createConfig } from '@wagmi/core'
import { arbitrum } from '@wagmi/core/chains'
import { walletConnect } from '@wagmi/connectors'

export const projectId = "c57e0bb001a4dc96b54b9ced656a3cb8"

export const wagmiConfig = createConfig({
    chains: [arbitrum],
    connectors: [
        walletConnect({
            projectId,
            metadata: {
                name: 'Autonomi',
                description: 'Autonomi Network Desktop App',
                url: 'https://autonomi.com',
                icons: ['https://avatars.githubusercontent.com/u/179229932?s=200&v=4'],
            },
            showQrModal: true,
            qrModalOptions: {
                themeMode: 'dark',
                themeVariables: {
                    '--wcm-z-index': '9999'
                }
            }
        })
    ],
    transports: {
        [arbitrum.id]: http(),
    },
})

// For backward compatibility
export const networks = [arbitrum]
export const wagmiAdapter = { wagmiConfig }
