export function splitTypstSvgByPage(svgText, options = {}) {
    if (typeof svgText !== 'string' || svgText.length === 0) {
        console.warn('拆分SVG失败：输入必须是非空字符串');
        return [];
    }

    const {
        preserveWhitespace = true,
        strictMode = false
    } = (typeof options === 'object' && options !== null) ? options : {};

    const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';
    const DEFAULT_WIDTH = '596';
    const DEFAULT_HEIGHT = '842';
    const DEFAULT_STYLE = 'overflow: visible;';
    const SAFE_WHITESPACE_REGEX = /(?<=>)\n\s+(?=<)/g;

    let svgDoc, originalSvg;
    try {
        const parser = new DOMParser();
        svgDoc = parser.parseFromString(svgText, 'image/svg+xml');

        if (strictMode) {
            const parseError = svgDoc.querySelector('parsererror');
            const isInvalid = parseError?.parentNode === svgDoc.documentElement
                || svgDoc.documentElement.namespaceURI !== SVG_NAMESPACE;
            if (isInvalid) {
                console.warn('拆分SVG失败[严格模式]：无效的SVG格式');
                return [];
            }
        }

        originalSvg = svgDoc.querySelector('svg') || svgDoc.documentElement;
        if (!originalSvg || originalSvg.namespaceURI !== SVG_NAMESPACE) {
            console.warn('拆分SVG失败：未找到合法的SVG根元素');
            return [];
        }
    } catch (error) {
        console.warn('拆分SVG失败：解析异常', error);
        return [];
    }

    const getSafeAttr = (el, attr, defaultValue) => {
        const value = el.getAttribute(attr);
        return (typeof value === 'string' && value.trim() !== '') ? value.trim() : defaultValue;
    };

    const xmlns = getSafeAttr(originalSvg, 'xmlns', SVG_NAMESPACE);
    const xmlnsXlink = getSafeAttr(originalSvg, 'xmlns:xlink', 'http://www.w3.org/1999/xlink');
    const originalStyle = getSafeAttr(originalSvg, 'style', DEFAULT_STYLE);
    const originalWidth = getSafeAttr(originalSvg, 'width', DEFAULT_WIDTH);
    const originalHeight = getSafeAttr(originalSvg, 'height', DEFAULT_HEIGHT);

    const styleTags = Array.from(originalSvg.querySelectorAll('style'))
        .map(style => style.outerHTML)
        .join('');
    const defsTags = Array.from(originalSvg.querySelectorAll('defs'))
        .map(defs => defs.outerHTML)
        .join('');

    const pageGroups = Array.from(originalSvg.querySelectorAll('.typst-page'));
    const pageCount = pageGroups.length;
    if (pageCount === 0) {
        console.warn('拆分SVG失败：未找到.typst-page分页节点');
        return [];
    }

    const whitespaceRegex = preserveWhitespace ? null : SAFE_WHITESPACE_REGEX;

    const escapeSvg = (str) => {
        if (typeof str !== 'string') return str;
        return str.replace(/"/g, '&quot;').replace(/&/g, '&amp;');
    };

    const prefixStart = `<svg xmlns="${escapeSvg(xmlns)}" xmlns:xlink="${escapeSvg(xmlnsXlink)}" width="`;
    const prefixMiddle = `" height="`;
    const prefixEnd = `" style="${escapeSvg(originalStyle)}">${styleTags}${defsTags}`;
    const suffix = `</svg>`;

    const pageSvgArray = new Array(pageCount);
    for (let i = 0; i < pageCount; i++) {
        const pageGroup = pageGroups[i];
        if (!pageGroup || !(pageGroup instanceof Element)) {
            console.warn(`拆分SVG警告：第${i+1}页节点无效，跳过`);
            pageSvgArray[i] = '';
            continue;
        }

        const pageWidth = getSafeAttr(pageGroup, 'data-page-width', originalWidth);
        const pageHeight = getSafeAttr(pageGroup, 'data-page-height', originalHeight);

        let pageContent;
        const transformAttr = getSafeAttr(pageGroup, 'transform', '');
        if (transformAttr) {
            const clone = pageGroup.cloneNode(true);
            clone.setAttribute('transform', 'translate(0, 0)');
            pageContent = clone.outerHTML;
        } else {
            pageContent = pageGroup.outerHTML;
        }

        let singlePageSvg = [
            prefixStart, escapeSvg(pageWidth),
            prefixMiddle, escapeSvg(pageHeight),
            prefixEnd, pageContent,
            suffix
        ].join('').trim();

        if (!preserveWhitespace && whitespaceRegex) {
            singlePageSvg = singlePageSvg.replace(whitespaceRegex, ' ');
        }

        if (strictMode) {
            const testParser = new DOMParser();
            const testDoc = testParser.parseFromString(singlePageSvg, 'image/svg+xml');
            const testError = testDoc.querySelector('parsererror');
            if (testError?.parentNode === testDoc.documentElement) {
                console.warn(`拆分SVG警告：第${i+1}页SVG生成非法，使用默认模板`);
                singlePageSvg = `${prefixStart}${DEFAULT_WIDTH}${prefixMiddle}${DEFAULT_HEIGHT}${prefixEnd}</svg>`;
            }
        }

        pageSvgArray[i] = singlePageSvg;
    }

    return pageSvgArray.filter(svg => svg !== '');
}