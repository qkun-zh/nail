import {
    // 光标移动相关
    cursorCharLeft,
    selectCharLeft,
    cursorGroupLeft,
    selectGroupLeft,
    cursorCharRight,
    selectCharRight,
    cursorGroupRight,
    selectGroupRight,
    cursorLineUp,
    selectLineUp,
    cursorLineDown,
    selectLineDown,
    cursorLineBoundaryBackward,
    selectLineBoundaryBackward,
    cursorDocStart,
    selectDocStart,
    cursorPageUp,
    selectPageUp,
    cursorPageDown,
    selectPageDown,
    cursorLineBoundaryForward,
    selectLineBoundaryForward,
    cursorDocEnd,
    selectDocEnd,

    // 文本删除相关
    deleteCharBackward,
    deleteCharForward,
    deleteGroupBackward,
    deleteGroupForward,
    deleteToLineEnd,
    deleteToLineStart,

    // 行操作相关
    deleteLine,
    moveLineDown,
    moveLineUp,
    copyLineDown,
    copyLineUp,
    selectLine,

    // 撤销重做相关
    redo,
    undo,
    undoSelection,

    // Tab 相关
    insertTab,
    indentLess
} from '@codemirror/commands';

import {
    // 搜索替换相关
    closeSearchPanel,
    gotoLine,
    openSearchPanel,
    replaceAll,
    selectMatches,
    selectNextOccurrence,
    selectSelectionMatches,
} from '@codemirror/search';

import {
    foldCode,
    unfoldCode,
    foldAll,
    unfoldAll,
} from '@codemirror/language';
import localForage from "localforage";
import {idAndVersion} from "$lib/func/article/getIdAndVersionOfArticleFromURL.js";

const saveLocally = (view) => {
    const {id,version} = idAndVersion();
    const articleStore = localForage.createInstance({name:"article",storeName:id});
    try {
        articleStore.setItem(version, view.state.doc.toString());
        alert("本地保存成功")
        return true;
    }catch (e){
        alert("本地保存失败")
        return false;
    }
};

const saveRemotely = (view)=>{
    const {id,version} = idAndVersion();
    try {
        alert("远程保存成功")
        return true;
    }catch (e){
        alert("远程保存失败：",e)
        return false;
    }
}

const post = (view)=>{
    const {id,version} = idAndVersion();
    try {
        alert("发布成功\n这篇文章将新增一个版本，该版本不可再修改\n点击“确定”后你将永久离开此编辑区并不可重返")
        return true;
    }catch (e){
        alert("发布失败：",e)
        return false;
    }
}


// 命名改为 nailKeymap，纯 JS 风格（移除 TypeScript 类型标注）
export const nailKeymap = [
    // ====================== 搜索替换 ======================
    { key: 'Mod-f', run: openSearchPanel, scope: 'article search-panel' },
    { key: 'Escape', run: closeSearchPanel, scope: 'article search-panel' },
    { key: 'Alt-Enter', run: selectMatches, scope: 'article search-panel' },
    { key: 'Mod-Alt-Enter', run: replaceAll, scope: 'article search-panel' },
    { key: 'Ctrl-g', run: gotoLine },
    { key: 'Mod-d', run: selectNextOccurrence, preventDefault: true },
    { key: 'Shift-Mod-l', run: selectSelectionMatches },

    // ====================== Tab 键绑定 ======================
    { key: 'Tab', run: insertTab, preventDefault: true },
    { key: 'Shift-Tab', run: indentLess, preventDefault: true },

    // ====================== 光标移动 ======================
    {
        key: 'ArrowLeft',
        run: cursorCharLeft,
        shift: selectCharLeft,
        preventDefault: true,
    },
    {
        key: 'Mod-ArrowLeft',
        mac: 'Alt-ArrowLeft',
        run: cursorGroupLeft,
        shift: selectGroupLeft,
    },
    {
        key: 'ArrowRight',
        run: cursorCharRight,
        shift: selectCharRight,
        preventDefault: true,
    },
    {
        key: 'Mod-ArrowRight',
        mac: 'Alt-ArrowRight',
        run: cursorGroupRight,
        shift: selectGroupRight,
    },
    {
        key: 'ArrowUp',
        run: cursorLineUp,
        shift: selectLineUp,
        preventDefault: true,
    },
    {
        key: 'ArrowDown',
        run: cursorLineDown,
        shift: selectLineDown,
        preventDefault: true,
    },
    {
        key: 'Home',
        run: cursorLineBoundaryBackward,
        shift: selectLineBoundaryBackward,
    },
    {
        mac: 'Cmd-ArrowLeft',
        run: cursorLineBoundaryBackward,
        shift: selectLineBoundaryBackward,
    },
    { key: 'Mod-Home', run: cursorDocStart, shift: selectDocStart },
    { mac: 'Cmd-ArrowUp', run: cursorDocStart, shift: selectDocStart },
    { key: 'PageUp', run: cursorPageUp, shift: selectPageUp },
    { mac: 'Ctrl-ArrowUp', run: cursorPageUp, shift: selectPageUp },
    { key: 'PageDown', run: cursorPageDown, shift: selectPageDown },
    { mac: 'Ctrl-ArrowDown', run: cursorPageDown, shift: selectPageDown },
    {
        key: 'End',
        run: cursorLineBoundaryForward,
        shift: selectLineBoundaryForward,
    },
    {
        mac: 'Cmd-ArrowRight',
        run: cursorLineBoundaryForward,
        shift: selectLineBoundaryForward,
    },
    { key: 'Mod-End', run: cursorDocEnd, shift: selectDocEnd },
    { mac: 'Cmd-ArrowDown', run: cursorDocEnd, shift: selectDocEnd },

    // Mac 系统光标移动兼容
    { mac: 'Ctrl-b', run: cursorCharLeft, shift: selectCharLeft, preventDefault: true },
    { mac: 'Ctrl-f', run: cursorCharRight, shift: selectCharRight },
    { mac: 'Ctrl-p', run: cursorLineUp, shift: selectLineUp },
    { mac: 'Ctrl-n', run: cursorLineDown, shift: selectLineDown },
    { mac: 'Ctrl-a', run: cursorLineBoundaryBackward, shift: selectLineBoundaryBackward },
    { mac: 'Ctrl-e', run: cursorLineBoundaryForward, shift: selectLineBoundaryForward },

    // ====================== 文本删除 ======================
    { key: 'Backspace', run: deleteCharBackward },
    { key: 'Delete', run: deleteCharForward },
    { key: 'Mod-Backspace', mac: 'Alt-Backspace', run: deleteGroupBackward },
    { key: 'Mod-Delete', mac: 'Alt-Delete', run: deleteGroupForward },
    { mac: 'Mod-Backspace', run: deleteToLineStart },
    { mac: 'Mod-Delete', run: deleteToLineEnd },
    { mac: 'Ctrl-d', run: deleteCharForward },
    { mac: 'Ctrl-h', run: deleteCharBackward },
    { mac: 'Ctrl-k', run: deleteToLineEnd },
    { mac: 'Ctrl-Alt-h', run: deleteGroupBackward },

    // ====================== 行操作 ======================
    { key: 'Shift-Mod-k', run: deleteLine },
    { key: 'Alt-ArrowDown', run: moveLineDown },
    { key: 'Alt-ArrowUp', run: moveLineUp },
    { win: 'Shift-Alt-ArrowDown', mac: 'Shift-Alt-ArrowDown', run: copyLineDown },
    { win: 'Shift-Alt-ArrowUp', mac: 'Shift-Alt-ArrowUp', run: copyLineUp },
    { key: 'Mod-l', run: selectLine, preventDefault: true },

    // ====================== 撤销重做 ======================
    { key: 'Mod-z', run: undo, preventDefault: true },
    { key: 'Mod-y', run: redo, preventDefault: true },
    { key: 'Mod-Shift-z', run: redo, preventDefault: true },
    { key: 'Mod-u', run: undoSelection, preventDefault: true },

    // ====================== 代码折叠 ======================
    { key: 'Ctrl-Shift-[', mac: 'Cmd-Alt-[', run: foldCode },
    { key: 'Ctrl-Shift-]', mac: 'Cmd-Alt-]', run: unfoldCode },
    { key: 'Mod-k Mod-0', run: foldAll },
    { key: 'Mod-k Mod-j', run: unfoldAll },

    // ====================== 自定义 ======================
    { key: 'Ctrl-s', run: saveLocally, preventDefault: true },
    { key: 'Alt-s', run: saveRemotely, preventDefault: true },
    { key: 'Ctrl-p', run: post, preventDefault: true },
];



