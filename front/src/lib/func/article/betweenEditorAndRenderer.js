import * as Comlink from "comlink";
import {EditorState,Text} from "@codemirror/state";
import {diffArrays} from 'diff';

import {collab, receiveUpdates} from "@codemirror/collab";
import {parse} from "./parse.js"
let localState = EditorState.create({doc:Text.empty,extensions:[collab()]});
let localVersion = 0;
let sendWrappedHtmlDiffProxy = null;
let docDiff = [];
let changeRecorder = null;
let diffRangeOnOld = [];
let diffRangeOnNew = [];
let affectedHtmlSnippet = [];
let id = 0;
let localHtmlRaw = [];
let htmlDiff =[];
let localHtmlFinalOld = [];
let localHtmlFinalNew = [];


function getId(){
    return ++id;
}

function updateLocalHtmlAndGetHtmlDiff(){
    updateLocalHtmlRaw();
    updateLocalHtmlFinal();
    getHtmlDiff();
    localHtmlFinalOld = localHtmlFinalNew;
}

function getHtmlDiff(){
    htmlDiff = diffArrays(localHtmlFinalOld, localHtmlFinalNew);
}
function updateLocalHtmlRaw(){
    let len = affectedHtmlSnippet.length;
    for(let i=0;i<len;++i){
        let [from,to] = diffRangeOnOld[i]
        localHtmlRaw.splice(from,to-from+1,...affectedHtmlSnippet[i])
    }
}
function updateLocalHtmlFinal() {
    const rawLen = localHtmlRaw.length;
    localHtmlFinalNew = [];
    const tempInlineArea = [];
    let lastTagType = "";

    for (let i = 0; i < rawLen; i++) {
        const rawItem = localHtmlRaw[i];
        const line = {
            data: rawItem.data,
            type: rawItem.type
        };
        const tagType = line.type;

        if (tagType === "div" || tagType === "br" || tagType === "p"||tagType==="hr") {
            const tempLen = tempInlineArea.length;
            if (tempLen > 0) {
                for (let j = 0; j < tempLen; j++) {
                    const item = tempInlineArea[j];
                    tempInlineArea[j] = item.type === "span" || item.type === "q"
                        ? `<${item.type}>${item.data}</${item.type}>`
                        : item.data;
                }
                localHtmlFinalNew.push(tempInlineArea.join(""));
                tempInlineArea.length = 0;
            }
            console.log(line.data);
            localHtmlFinalNew.push(line.data);
        } else if (tagType === "img" || tagType === "a") {
            tempInlineArea.push(line);
        } else {
            if (lastTagType === tagType) {
                tempInlineArea[tempInlineArea.length - 1].data += line.data;
            } else {
                tempInlineArea.push(line);
            }
        }
        lastTagType = tagType;
    }

    const finalTempLen = tempInlineArea.length;
    if (finalTempLen > 0) {
        for (let j = 0; j < finalTempLen; j++) {
            const item = tempInlineArea[j];
            tempInlineArea[j] = item.type === "span" || item.type === "q"
                ? `<${item.type}>${item.data}</${item.type}>`
                : item.data;
        }
        localHtmlFinalNew.push(tempInlineArea.join(""));
        tempInlineArea.length = 0;
    }
}
async function recv(wrappedDocDiff){
    unwrapWrappedDocDiff(wrappedDocDiff);
    recordDiffRangeOnOld();
    updateLocalStateAndChangeRecorder();
    recordDiffRangeOnNew();
    await parseAffectedDocSnippetToHtml();
    updateLocalHtmlAndGetHtmlDiff();
    await sendWrappedHtmlDiff()
    clearAllUseless();
}
function registerCallbackProxyOfRenderer(callbackProxyOfRenderer){
    sendWrappedHtmlDiffProxy = callbackProxyOfRenderer;
    return !!sendWrappedHtmlDiffProxy;
}

Comlink.expose({
    recv: recv,
    registerCallbackProxyOfRenderer:registerCallbackProxyOfRenderer});

function compress(htmlDiff) {
    let len=htmlDiff.length;
    for(let i=0;i<len;++i){
        let temp  = htmlDiff[i];
        if(temp.added===false){temp.value=[];}
    }
    return htmlDiff;
}

async function sendWrappedHtmlDiff() {
    if(diffRangeOnOld.length !== affectedHtmlSnippet.length){
        console.log("html diff do not match range")
        return;
    }

    for (let i = 0; i < 8; ++i) {
        if (sendWrappedHtmlDiffProxy) {
            sendWrappedHtmlDiffProxy({
                data: compress(htmlDiff),
                version: localVersion,
            })
            break;
        } else {
            await new Promise(r => setTimeout(r, 4));
        }
    }
}
function clearAllUseless(){
    docDiff = [];
    changeRecorder = null;
    diffRangeOnOld = [];
    diffRangeOnNew = [];
    affectedHtmlSnippet = [];
    htmlDiff =[];
    localHtmlFinalNew = [];
}



async function parseAffectedDocSnippetToHtml() {
    let doc = localState.doc;
    for (let i = 0; i < diffRangeOnNew.length; ++i) {
        let lineFrom = diffRangeOnNew[i][0];
        let lineTo = diffRangeOnNew[i][1];
        affectedHtmlSnippet.push(await parse(doc.iterLines(lineFrom, lineTo + 1), lineTo - lineFrom + 1));
    }
}
function recordDiffRangeOnOld(){
    diffRangeOnOld = [];
    if (!docDiff?.[0]?.changes || docDiff[0].changes.length === 0) {return diffRangeOnOld;}
    let changes = docDiff[0].changes;
    let len = changes.length;
    let doc = localState.doc;
    for(let i=0;i<len;++i){
        let change = changes[i];
        let lineFrom  = doc.lineAt(change.from).number-1;
        let lineTo = doc.lineAt(change.to).number-1
        if(diffRangeOnOld.length!==0){
            let lastRange = diffRangeOnOld[diffRangeOnOld.length-1];
            if(lastRange[1]+1>=lineFrom){
                lastRange[1] = Math.max(lastRange[1],lineTo);
            }else{
                diffRangeOnOld.push([lineFrom,lineTo]);
            }
        }else{
            diffRangeOnOld.push([lineFrom,lineTo]);
        }
    }
}
function recordDiffRangeOnNew(){
    diffRangeOnNew = [];
    let doc = localState.doc;
    changeRecorder.iterChanges((_1, _2, from, to, _3) => {
        let lineFrom  =doc.lineAt(from).number;
        let lineTo = doc.lineAt(to).number
        if(diffRangeOnNew.length!==0){
            let lastRange = diffRangeOnNew[diffRangeOnNew.length-1];
            if(lastRange[1]+1>=lineFrom){
                lastRange[1] = Math.max(lastRange[1],lineTo);
            }else{
                diffRangeOnNew.push([lineFrom,lineTo]);
            }
        }else{
            diffRangeOnNew.push([lineFrom,lineTo]);
        }
    })
}
function updateLocalStateAndChangeRecorder(){
    let tr = receiveUpdates(localState,docDiff);
    changeRecorder = tr.changes;
    localState = tr.state;
}
function unwrapWrappedDocDiff(wrappedDocDiff){
    if(!checkIfVersionLegal(wrappedDocDiff.version)){
        console.log("id or version not match")
        return []
    }
    for(let i=0;i<wrappedDocDiff.data.changes.length;++i){
        wrappedDocDiff.data.changes[i].insert = Text.of(wrappedDocDiff.data.changes[i].insert);
    }
    docDiff =  [wrappedDocDiff.data];
}
function checkIfVersionLegal(remoteVersion) {
    if (remoteVersion <= localVersion) {
        return false;
    } else {
        localVersion = remoteVersion;
        return true;
    }
}
