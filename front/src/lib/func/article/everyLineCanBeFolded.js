// import {foldService} from "@codemirror/language";
//
// export function everyLineCanBeFolded() {
//     return foldService.of((state, lineStart, _) => {
//         const line = state.doc.lineAt(lineStart);
//         return { from: line.from, to: line.to };
//     })
// }


import { foldService,foldable } from "@codemirror/language";

export function everyLineCanBeFolded() {
    return foldService.of((state, lineFrom, lineTo) => {return { from: lineFrom, to: lineTo};});
}