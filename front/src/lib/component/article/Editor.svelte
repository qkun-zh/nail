<script>
    import { onMount, onDestroy } from 'svelte';
    import { EditorState, Text } from "@codemirror/state";
    import { drawSelection, EditorView, dropCursor, lineNumbers, highlightActiveLineGutter, highlightWhitespace, keymap } from "@codemirror/view";
    import { history } from "@codemirror/commands";
    import { search } from '@codemirror/search';
    import { limitSomething } from "$lib/func/article/limitSomething.js";
    import { syncDiff } from '$lib/func/article/sync_diff_between_editor_and_parser.js';
    import { syncInit } from "$lib/func/article/sync_init_between_editor_and_parser.js";
    import { collab } from "@codemirror/collab";
    import { getContext } from "svelte";
    import { nailKeymap } from "$lib/func/article/nailKeymap.js";
    import { dropImage } from "$lib/func/article/dropImage.js";
    import { foldGutter } from "@codemirror/language"
    import { nml } from "$lib/func/article/nmlHighlight.js";
    import { everyLineCanBeFolded } from "$lib/func/article/everyLineCanBeFolded.js";
    import { editorTheme } from "$lib/func/article/editorTheme.js";
    import { editorStateField } from "$lib/func/article/editorStateField.js";

    let { editorConfig = $bindable({}) } = $props();

    let editorViewParent = null;
    let editorView = null;

    let editorState = null;
    let parserWorkerProxy = null;

    onMount(() => {
        if (editorView) {
            editorView.destroy();
            editorView = null;
        }

        parserWorkerProxy = getContext(editorConfig.parserWorkerProxyKey);

        const ext = editorConfig.writable === true ? [
            drawSelection(),
            EditorView.lineWrapping,
            highlightWhitespace(),
            lineNumbers(),
            highlightActiveLineGutter(),
            foldGutter({ openText: "-", closedText: "+" }),
            history(),
            everyLineCanBeFolded(),
            search(),
            dropCursor(),
            editorTheme(),
            dropImage(),
            editorStateField,
            nml,
            keymap.of(nailKeymap),
            collab({ startVersion: 0 }),
            limitSomething(),
            syncInit({ initDoc: editorConfig.initDoc }),
            syncDiff(parserWorkerProxy),
        ] : [
            drawSelection(),
            EditorView.lineWrapping,
            highlightWhitespace(),
            lineNumbers(),
            foldGutter({ openText: "-", closedText: "+" }),
            everyLineCanBeFolded(),
            search(),
            editorTheme(),
            editorStateField,
            nml,
            keymap.of(nailKeymap),
            collab({ startVersion: 0 }),
            limitSomething(),
            syncInit({ initDoc: editorConfig.initDoc }),
            syncDiff(parserWorkerProxy),
            EditorState.readOnly.of(true),
            EditorView.editable.of(false),
        ];

        editorState = EditorState.create({
            doc: Text.empty,
            extensions: ext,
        });

        editorView = new EditorView({
            state: editorState,
            parent: editorViewParent,
        });

        editorView.focus();
    });

    onDestroy(() => {

        if (editorView) {
            editorView.destroy();
            editorView = null;
        }

        editorState = null;

        parserWorkerProxy = null;

        editorViewParent = null;
    });
</script>

<div bind:this={editorViewParent}></div>
