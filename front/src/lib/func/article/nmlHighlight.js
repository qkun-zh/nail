import { StreamLanguage, syntaxHighlighting, HighlightStyle, LanguageSupport,ParseContext } from "@codemirror/language";
import { Tag } from "@lezer/highlight";
const syntaxMarkerTag = Tag.define();
const escapeComboTag = Tag.define();

const nmlHighlightStyle = HighlightStyle.define([
    { tag: syntaxMarkerTag, color: "rgba(216,4,4,0.75)",},
    { tag: escapeComboTag, color: "#062ded" }
]);

const createInitialState = () => ({
    markerNum: 0,
    isFirstChar: true
});

const nmlParser = {
    name: "nml",
    startState: createInitialState,
    copyState: (s) => ({ ...s }),

    token(stream, state) {
        if (stream.sol()) {
            Object.assign(state, createInitialState());
        }
        if (stream.eol()) return null;

        if (state.markerNum === 0) {
            if (state.isFirstChar && stream.match(/^\\\|/)) {
                state.isFirstChar = false;
                return "escapeCombo";
            }

            if (state.isFirstChar && stream.match(/^\|/)) {
                state.markerNum = 1;
                state.isFirstChar = false;
                return "syntaxMarker";
            }

            // if(!state.isFirstChar){
            //     stream.skipToEnd();
            // }
            stream.next();
            state.isFirstChar = false;
            return null;
        }

        if (state.markerNum === 1) {
            if (stream.match(/^\\\|/)) {
                return "escapeCombo";
            }

            if (stream.match(/^\|/)) {
                state.markerNum = 2;
                return "syntaxMarker";
            }

            stream.next();
            return null;
        }

        if (state.markerNum === 2) {
            stream.skipToEnd();
            return null;
        }

        stream.next();
        return null;
    },

    tokenTable: {
        syntaxMarker: syntaxMarkerTag,
        escapeCombo: escapeComboTag
    }
};

export const nml = new LanguageSupport(
    StreamLanguage.define(nmlParser),
    [syntaxHighlighting(nmlHighlightStyle)]
).extension;