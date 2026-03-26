import {ViewPlugin} from "@codemirror/view";
import {xxhash128} from "hash-wasm";
import {Text} from "@codemirror/state";
import localForage from "localforage";

export function dropImage() {
    return ViewPlugin.fromClass(class {
        constructor(view) {
            this.imgStoreBase64 = localForage.createInstance({name:'image',storeName:"base64"})
            this.imgStorelocalUrl = localForage.createInstance({name:'image',storeName:"localUrl"})

            this.view = view;
            this.dropHandler = async (e) => {
                e.preventDefault();
                const files = e.dataTransfer.files;
                const filesNum = files.length;
                if (filesNum === 0) return;

                let from = this.view.state.doc.lineAt(this.view.posAtCoords({x: e.x, y: e.y}, false)).to;

                let imageFile;
                let imageFileArray = [];
                let maxSize = 0;
                for (let i = 0; i < filesNum; ++i) {
                    imageFile = files[i];
                    let size = imageFile.size;
                    maxSize = maxSize>size?maxSize:size;
                    if (imageFile.type && imageFile.type.indexOf('image/') === 0) {
                        imageFileArray.push(imageFile);
                    }
                }

                let iLen = imageFileArray.length;
                if(iLen<=0){} else if(iLen>8){
                    alert("单次插入的图片总数量不能大于8张\n本次图片插入被拒绝");
                }else if(maxSize>1000*1000*2){
                    alert("任何一张图片的体积都不能大于2MB\n本次图片插入被拒绝")
                } else{
                    const combinedArray = await Promise.all(
                        imageFileArray.map(async (file) => {
                            const fileU8a = new Uint8Array(await file.arrayBuffer());
                            const hash = await xxhash128(fileU8a, 0, 0);
                            const localUrl = URL.createObjectURL(file);
                            const base64 = await new Promise((resolve) => {
                                const reader = new FileReader();
                                reader.readAsDataURL(file);
                                reader.onload = () => resolve(reader.result);
                            });

                            return { hash, localUrl,base64};
                        })
                    );
                    const imageHashArray = combinedArray.map(item => item.hash);
                    const imageLocalUrlArray = combinedArray.map(item => item.localUrl);
                    const imageBase64Array = combinedArray.map(item => item.base64);
                    let len = imageHashArray.length
                    let imgHtml = [''];
                    for(let i=0;i<len;++i){
                        await this.imgStorelocalUrl.setItem(imageHashArray[i],imageLocalUrlArray[i]);
                        await this.imgStoreBase64.setItem(imageHashArray[i],imageBase64Array[i]);
                        imgHtml.push(`||${imageHashArray[i]}`);
                    }
                    if(len>0){
                        console.log("imgHtml:",imgHtml)
                        this.view.dispatch({
                            changes: {
                                from: from,
                                to: from,
                                insert: Text.of(imgHtml)
                            }
                        });
                    }
                }
            };

            this.view.dom.addEventListener('drop', this.dropHandler);
        }

        destroy() {
            this.view.dom.removeEventListener('drop', this.dropHandler);
            this.dropHandler = null;
            this.view = null;
        }
    });
}