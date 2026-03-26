import localForage from "localforage";
import {getContext} from "svelte";

const SYNTAX_SYMBOL = Object.freeze({
    ESCAPER: `\\`,
    MARKER: `|`,
});

const ESCAPE_RULES = Object.freeze({
    PRE_SEPARATOR: new Map([[`&`, `&amp`], [`<`, `&lt`]]),
    POST_SEPARATOR: new Map([[`&`, `&amp`], [`<`, `&lt`], [`>`, `&gt`], [`"`, `&quot`]]),
});

const MARKER_NUM = Object.freeze({
    ZERO: 0,
    ONE: 1,
    TWO: 2,
});

const renderHelpers = {
    lineBreak: () => {return {data:`<br>`,type:"br"}},
    emptyDiv: () => {return {data:`<div></div>`,type:"div"}},
    dividingLine: () => {return {data:`<hr>`,type:"hr"}},
    parserError: () => {return {data:`<div></div>`,type:"div"}},
};

// const imgStore  = localForage.createInstance({name:"imgStore"});
const imgStoreBase64 = localForage.createInstance({name:'image',storeName:"base64"})
const imgStoreLocalUrl = localForage.createInstance({name:'image',storeName:"localUrl"})

async function parseLine(text) {
    let shortCircuit = false;
    let isFirstChar = true;
    let markerNum = 0;
    let x = -1;

    let preSeparator = [];
    let postSeparator = [];

    let preSeparatorEmpty = true;
    let postSeparatorEmpty = true;

    const textLen = text.length;
    if (textLen === 0) {
        x = 0;
    }

    const markerHandlers = new Map([
        [MARKER_NUM.ZERO, handleZeroMarkers],
        [MARKER_NUM.ONE, handleOneMarker],
        [MARKER_NUM.TWO, handleTwoMarkers]
    ]);

    function handleZeroMarkers(i) {
        const char = text[i];

        if (isFirstChar && char === SYNTAX_SYMBOL.ESCAPER && text[i + 1] === SYNTAX_SYMBOL.MARKER) {
            preSeparator.push(SYNTAX_SYMBOL.MARKER);
            isFirstChar = false;
            i++;
            return i;
        }

        if (char === SYNTAX_SYMBOL.MARKER && isFirstChar) {
            markerNum++;
            if (textLen === 1) {
                x = 1;
                preSeparator.push(renderHelpers.emptyDiv());
            }
            return i;
        }

        if (ESCAPE_RULES.PRE_SEPARATOR.has(char)) {
            preSeparator.push(ESCAPE_RULES.PRE_SEPARATOR.get(char));
            isFirstChar = false;
            return i;
        }

        preSeparator.push(char);
        isFirstChar = false;
        return i;
    }

    function handleOneMarker(i) {
        const char = text[i];

        if (char === SYNTAX_SYMBOL.ESCAPER && text[i + 1] === SYNTAX_SYMBOL.MARKER) {
            preSeparator.push(SYNTAX_SYMBOL.MARKER);
            i++;
            preSeparatorEmpty = false;
            return i;
        }

        if (char === SYNTAX_SYMBOL.MARKER) {
            markerNum++;
            if (textLen === 2) {
                x = 2;
                preSeparator.push(renderHelpers.dividingLine());
            }
            return i;
        }

        if (ESCAPE_RULES.PRE_SEPARATOR.has(char)) {
            preSeparator.push(ESCAPE_RULES.PRE_SEPARATOR.get(char));
            preSeparatorEmpty = false;
            return i;
        }

        preSeparator.push(char);
        preSeparatorEmpty = false;
        return i;
    }

    function handleTwoMarkers(i) {
        const char = text[i];

        if (textLen === 2) {
            postSeparator.push(renderHelpers.dividingLine());
            x = 2;
            return i;
        }

        if (char.codePointAt(0) < 0 || char.codePointAt(0) > 127) {
            shortCircuit = true;
            return i;
        }

        if (ESCAPE_RULES.POST_SEPARATOR.has(char)) {
            postSeparator.push(ESCAPE_RULES.POST_SEPARATOR.get(char));
        } else {
            postSeparator.push(char);
        }

        postSeparatorEmpty = false;
        return i;
    }

    let i = 0;
    for (; i < textLen; i++) {
        i = markerHandlers.get(markerNum)(i);
    }

    if (shortCircuit) {
        return renderHelpers.parserError();
    }

    switch (x) {
        case 0:
            return renderHelpers.lineBreak();
        case 1:
            return renderHelpers.dividingLine();
        case 2:
            return renderHelpers.emptyDiv();
        default:
            break;
    }

    const preStr = preSeparator.join("");
    const postStr = postSeparator.join("");

    if (markerNum === MARKER_NUM.ZERO) {
        return {data:`<p>${preStr}</p>`,type:"p"};
    } else if (markerNum === MARKER_NUM.ONE) {
        return {data:preStr,type:"span"};
    } else {
        if (!preSeparatorEmpty && !postSeparatorEmpty) {
            return {data:`<a href="${postStr}" target="_blank" rel="noopener noreferrer">${preStr}</a>`,type:"a"};
        } else if (preSeparatorEmpty && !postSeparatorEmpty) {
            let trimmedSrc = postStr.trim();
            if (trimmedSrc.startsWith('data:image/')) {

            } else if (trimmedSrc.startsWith('https://') || trimmedSrc.startsWith('http://')) {

            } else{
                // todo 先远程还是本地是个问题，也许需要文章判定先？？？
                let localUrl = await imgStoreLocalUrl.getItem(trimmedSrc);
                if(localUrl){//未本地保存
                    trimmedSrc = localUrl
                }else {//已经本地保存， 未远程保存
                    trimmedSrc = await imgStoreBase64.getItem(trimmedSrc);
                }//............
            }
            return {data:`<img src="${trimmedSrc}" loading="lazy" decoding="async">`,type:"img"};
        } else {
            return {data:preStr,type:"q"};
        }

    }
}

export async function parse(docLinesIter, len) {
    const html = [];
    for (const line of docLinesIter) {
        html.push(await parseLine(line));}
    return  html;
}