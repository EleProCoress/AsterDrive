<script setup lang="ts">
import { useData } from "vitepress"
import DefaultTheme from "vitepress/theme"
import { nextTick, onBeforeUnmount, onMounted, provide, ref } from "vue"

const { isDark } = useData()
const { Layout } = DefaultTheme
const zoomedDiagram = ref("")

function canUseViewTransition() {
	return (
		typeof document !== "undefined" &&
		"startViewTransition" in document &&
		window.matchMedia("(prefers-reduced-motion: no-preference)").matches
	)
}

provide("toggle-appearance", async ({ clientX: x, clientY: y }: MouseEvent) => {
	if (!canUseViewTransition()) {
		isDark.value = !isDark.value
		return
	}

	const endRadius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y))
	const clipPath = [`circle(0px at ${x}px ${y}px)`, `circle(${endRadius}px at ${x}px ${y}px)`]

	await document.startViewTransition(async () => {
		isDark.value = !isDark.value
		await nextTick()
	}).ready

	document.documentElement.animate(
		{ clipPath: isDark.value ? clipPath.reverse() : clipPath },
		{
			duration: 360,
			easing: "cubic-bezier(0.22, 1, 0.36, 1)",
			fill: "forwards",
			pseudoElement: `::view-transition-${isDark.value ? "old" : "new"}(root)`,
		},
	)
})

function closeDiagramZoom() {
	zoomedDiagram.value = ""
	document.body.classList.remove("aster-diagram-zoom-open")
}

function openDiagramZoom(event: MouseEvent) {
	const target = event.target instanceof Element ? event.target : null
	const diagram = target?.closest(".vp-doc .mermaid")

	if (!(diagram instanceof HTMLElement)) {
		return
	}

	const svg = diagram.querySelector("svg")
	if (!svg) {
		return
	}

	zoomedDiagram.value = svg.outerHTML
	document.body.classList.add("aster-diagram-zoom-open")
}

function closeDiagramZoomOnEscape(event: KeyboardEvent) {
	if (event.key === "Escape") {
		closeDiagramZoom()
	}
}

onMounted(() => {
	document.addEventListener("click", openDiagramZoom)
	document.addEventListener("keydown", closeDiagramZoomOnEscape)
})

onBeforeUnmount(() => {
	document.removeEventListener("click", openDiagramZoom)
	document.removeEventListener("keydown", closeDiagramZoomOnEscape)
	document.body.classList.remove("aster-diagram-zoom-open")
})
</script>

<template>
	<Layout />
	<Teleport to="body">
		<div v-if="zoomedDiagram" class="aster-diagram-zoom" role="dialog" aria-modal="true" @click.self="closeDiagramZoom">
			<button class="aster-diagram-zoom-close" type="button" aria-label="Close diagram zoom" @click="closeDiagramZoom">
				×
			</button>
			<div class="aster-diagram-zoom-canvas" v-html="zoomedDiagram" />
		</div>
	</Teleport>
</template>

<style>
::view-transition-old(root),
::view-transition-new(root) {
	animation: none;
	mix-blend-mode: normal;
}

::view-transition-old(root),
.dark::view-transition-new(root) {
	z-index: 1;
}

::view-transition-new(root),
.dark::view-transition-old(root) {
	z-index: 9999;
}

.VPSwitchAppearance {
	width: 22px !important;
}

.VPSwitchAppearance .check {
	transform: none !important;
}
</style>
