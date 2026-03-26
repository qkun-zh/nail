
import {EditorView} from "@codemirror/view";

export function editorTheme() {
    return EditorView.theme({
        "&.cm-focused .cm-cursor": {
            borderLeftColor: "#9c1010",
            borderLeftStyle: "dashed",
        },
        ".cm-activeLineGutter ": {
            backgroundColor: "#1B1B1B59",
            color: "#07FF00FF",
        },

    });
}