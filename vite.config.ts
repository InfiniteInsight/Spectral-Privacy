import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;
const port = process.env.PORT ? parseInt(process.env.PORT) : 5737;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],

	// Vite options tailored for Tauri development
	clearScreen: false,
	server: {
		port,
		strictPort: false, // Allow auto-increment (dev.sh will sync the port)
		host: host || false,
		hmr: host
			? {
					protocol: 'ws',
					host,
					port: 5738
				}
			: undefined,
		watch: {
			// Tell Vite to ignore watching `src-tauri`
			ignored: ['**/src-tauri/**']
		}
	},

	// Environment variables starting with TAURI_ are exposed to the client
	envPrefix: ['VITE_', 'TAURI_']
});
