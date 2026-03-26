import {EditorState, StateField} from "@codemirror/state";
import {editorStateField} from "$lib/func/article/editorStateField.js";

export function limitSomething(limitConfig={maxNumOfLines:10000-1,maxNumOfChars:10000*100-1}) {
    const { finalMaxLines, finalMaxChars } = validateLimitConfig(limitConfig);
    return EditorState.transactionFilter.of((tr) => {

        if (!tr.docChanged || tr.startState.doc.eq(tr.state.doc)) return tr;

        const maxNumOfImage = tr.state.field(editorStateField).maxNumOfImg;
        if(tr.newDoc.lines > finalMaxLines){
            alert("最大行数: "+finalMaxLines);
        }else if(tr.newDoc.length > finalMaxChars){
            alert("最大字数: "+finalMaxChars);
        }else if(isImgNumOverflow(tr,maxNumOfImage)){
            alert("最大图片数量："+maxNumOfImage);
        }else{
            return tr;
        }

        return [];
    });
}



function isImgNumOverflow(tr,maxNumOfImage){
    return tr.state.field(editorStateField).maybeNowNumOfImg > maxNumOfImage;
}

function validateLimitConfig(limitConfig) {
    let finalMaxLines = 10000;
    let finalMaxChars = finalMaxLines*100;

    if (limitConfig !== undefined && limitConfig !== null) {
        if (typeof limitConfig === 'object') {
            if ('maxNumOfLines' in limitConfig) {
                const lineValue = limitConfig.maxNumOfLines;
                if (typeof lineValue === 'number' && !isNaN(lineValue) && lineValue >= 0) {
                    finalMaxLines = lineValue;
                } else {
                    console.warn('[limitNumberOfLinesAndChars] maxNumOfLines 必须是非负数字，已设为不限制');
                }
            }

            if ('maxNumOfChars' in limitConfig) {
                const charValue = limitConfig.maxNumOfChars;
                if (typeof charValue === 'number' && !isNaN(charValue) && charValue >= 0) {
                    finalMaxChars = charValue;
                } else {
                    console.warn('[limitNumberOfLinesAndChars] maxNumOfChars 必须是非负数字，已设为不限制');
                }
            }
        } else {
            console.error('[limitNumberOfLinesAndChars] limitConfig 必须是对象，已设为不限制所有内容');
        }
    }

    return { finalMaxLines, finalMaxChars };
}