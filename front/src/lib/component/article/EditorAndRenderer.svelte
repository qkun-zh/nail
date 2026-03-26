<script>
    import Loading from "$lib/component/Loading.svelte";
    import Editor from '$lib/component/article/Editor.svelte';
    import Renderer from "$lib/component/article/Renderer.svelte";
    import WorkerConstructor from '$lib/func/article/betweenEditorAndRenderer.js?worker';

    import { wrap, releaseProxy } from "comlink";
    import {onDestroy, onMount, setContext} from 'svelte';


    let { config = $bindable({showEditor: true, showRenderer: true, writable: true}) } = $props();

    let parserWorker = null;
    let parserWorkerProxyKey  = Symbol("parserWorkerProxyKey");
    let parserWorkerProxyVal = null;
    let isEditorReady = $state(false);
    let isRendererReady = $state(false);
    let editorConfig = $state({
        initDoc:config.initDoc,
        parserWorkerProxyKey: parserWorkerProxyKey,
        writable:config.writable,
    });
    let rendererConfig = $state({
        parserWorkerProxyKey: parserWorkerProxyKey,
    })
    let displayEditor = $state('none');
    let displayRenderer = $state('none');
    let resizeEditor = $state('none');

    onMount(async () => {
        if (config.showEditor) {
            displayEditor = 'inline-block';
        } else {
            displayEditor = 'none';
        }
        if (config.showRenderer) {
            resizeEditor = 'horizontal'
            displayRenderer = 'inline-block';
        } else {
            resizeEditor = 'none'
            displayRenderer = 'none';
        }

        parserWorker = new WorkerConstructor();
        parserWorkerProxyVal = wrap(parserWorker);
        setContext(parserWorkerProxyKey, parserWorkerProxyVal)
        isRendererReady = true;
        isEditorReady = true;
    })


    onDestroy(()=>{
        if (parserWorkerProxyVal) {
            void parserWorkerProxyVal[releaseProxy]();
            parserWorkerProxyVal = null;
        }
        if (parserWorker) {
            parserWorker.terminate();
            parserWorker = null;
        }
        parserWorkerProxyKey = null;
        isEditorReady = false;
    })

</script>



{#if isEditorReady && isRendererReady}
    <div class="editor-renderer-container">
        <div class="editor-panel" style="display: {displayEditor};resize: {resizeEditor};">
            <Editor bind:editorConfig={editorConfig} />
        </div>
        <div class="renderer-panel" style="display: {displayRenderer}">
            <Renderer bind:rendererConfig={rendererConfig}></Renderer>
        </div>
    </div>
{:else}
    <Loading></Loading>
{/if}

<style>
    .editor-renderer-container {
        display: flex;
        width: 100%;
        height: 100vh;
        overflow: clip;
    }

    .editor-panel {
        height: 100%;
        width: 35%;
        resize: horizontal;
        overflow-x: clip;
        overflow-y: scroll;
    }

    .renderer-panel {
        flex: 1;
        height: 100%;
        overflow-x: clip;
        overflow-y: clip;
    }

</style>

