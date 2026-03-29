<script lang="ts">
	import Nav from '$lib/components/Nav.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import AiPane from '$lib/components/ai/AiPane.svelte';

	let { children } = $props();
	let aiPaneOpen = $state(false);
</script>

<Nav onToggleAi={() => aiPaneOpen = !aiPaneOpen} />
<main>
	{@render children()}
</main>
<Footer />
<AiPane bind:open={aiPaneOpen} />

<style>
	main {
		min-height: calc(100vh - 3.5rem - 8rem);
	}
</style>
