import { ViewPlugin } from "@codemirror/view";

export function syncInit(wrappedInitDoc) {
    let hasInit = false;
    return ViewPlugin.fromClass(class {
        constructor(view) {
            queueMicrotask(async () => {
                if (!view.destroyed && !hasInit) {
                    view.dispatch({
                        changes: {
                            from: 0,
                            to: view.state.doc.length,
                            insert: await wrappedInitDoc.initDoc
                        }
                    });
                    hasInit = true;
                }
            });
        }
        destroy() {
            hasInit = null;
        }
    });
}