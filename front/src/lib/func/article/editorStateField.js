import {StateField} from "@codemirror/state";

export const editorStateField = StateField.define({
    create(state) {
        return {nowNumOfImg:0,maxNumOfImg:99,maybeNowNumOfImg:0};
    },
    update(value, tr) {
        if (tr.docChanged && !tr.startState.doc.eq(tr.state.doc)){
            updateNowNumOfImg(value,tr);
        }

        return value;
    },
});

function updateNowNumOfImg(value,tr){
    if(value.maybeNowNumOfImg>value.maxNumOfImg){
        value.maybeNowNumOfImg=value.nowNumOfImg;
    }

    let docA = tr.startState.doc;
    let docB = tr.state.doc;
    let imgNumA = 0;
    let imgNumB = 0;
    tr.changes.iterChanges((fromA,toA,fromB,toB,inserted)=>{
        let lineFromA = docA.lineAt(fromA).number;
        let lineToA = docA.lineAt(toA).number;
        let lineIterA = docA.iterLines(lineFromA,lineToA+1);
        for(const line of lineIterA) {
            if(line.length>2&&line.startsWith("||")){
                ++imgNumA;
            }
        }

        let lineFromB = docB.lineAt(fromB).number;
        let lineToB = docB.lineAt(toB).number;
        let lineIterB = docB.iterLines(lineFromB,lineToB+1);
        for(const line of lineIterB) {
            if(line.length>2&&line.startsWith("||")){
                ++imgNumB;
            }
        }
    })
    let newNowNumOfImg = value.nowNumOfImg+(-imgNumA)+(+imgNumB);
    value.maybeNowNumOfImg = newNowNumOfImg;
    if(newNowNumOfImg<=value.maxNumOfImg){
        value.nowNumOfImg = newNowNumOfImg;
    }

}