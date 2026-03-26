const SVG_CLEAN_CONFIG = {
    keepAttrs: new Set(['viewBox', 'width', 'height', 'xmlns', 'xmlns:xlink', 'style', 'id', 'fill', 'stroke']),
    keepClassTags: new Set(['image', 'use', 'path', 'text', 'g']),
    removeDebugElements: true,
    removeWhitespaceNodes: true,
    styleCleanStrategy: 'remove-rule',
    styleDebugReg: /\.tsel/,
    debugSelector: '.tsel, .typst-content-hint, .text-guard',
    svgNamespace: 'http://www.w3.org/2000/svg',
    tags: {
        script: 'script',
        style: 'style',
        defs: 'defs'
    },
    emptyClipPathSelector: 'clipPath:empty'
};

export function cleanTypstRawSvgText(typstRawSvgText) {
    if (!typstRawSvgText || typeof typstRawSvgText !== 'string') return '';

    let svgDoc, svg;
    try {
        const parser = new DOMParser();
        svgDoc = parser.parseFromString(typstRawSvgText, 'image/svg+xml');

        const parserError = svgDoc.querySelector('parsererror');
        if (parserError?.parentNode === svgDoc.documentElement) {
            return '';
        }

        svg = svgDoc.querySelector('svg');
        if (!svg) return '';
    } catch (error) {
        console.warn('SVG解析失败', error);
        return '';
    }

    if (!svg.hasAttribute('xmlns')) {
        svg.setAttribute('xmlns', SVG_CLEAN_CONFIG.svgNamespace);
    }

    const trash = svgDoc.createDocumentFragment();

    Array.from(svg.getElementsByTagName(SVG_CLEAN_CONFIG.tags.script))
        .forEach(script => trash.appendChild(script));

    Array.from(svg.getElementsByTagName(SVG_CLEAN_CONFIG.tags.style))
        .forEach(style => {
            if (SVG_CLEAN_CONFIG.styleCleanStrategy === 'remove-tag' && SVG_CLEAN_CONFIG.styleDebugReg.test(style.textContent)) {
                trash.appendChild(style);
            } else if (SVG_CLEAN_CONFIG.styleCleanStrategy === 'remove-rule') {
                style.textContent = style.textContent.replace(
                    new RegExp(`[^}]*${SVG_CLEAN_CONFIG.styleDebugReg.source}[^}]*\\{[^}]*\\}`, 'g'),
                    ''
                );
                if (!style.textContent.trim()) {
                    trash.appendChild(style);
                }
            }
        });

    if (SVG_CLEAN_CONFIG.removeDebugElements) {
        Array.from(svg.querySelectorAll(SVG_CLEAN_CONFIG.debugSelector))
            .forEach(debugEl => trash.appendChild(debugEl));
    }

    Array.from(svg.querySelectorAll('[class]')).forEach(el => {
        const tagName = el.tagName.toLowerCase();
        const classValue = el.getAttribute('class')?.trim() || '';

        if (!SVG_CLEAN_CONFIG.keepClassTags.has(tagName) || !classValue) {
            el.removeAttribute('class');
        }
    });

    const defs = svg.querySelector(SVG_CLEAN_CONFIG.tags.defs);
    if (defs) {
        Array.from(defs.querySelectorAll(SVG_CLEAN_CONFIG.emptyClipPathSelector))
            .forEach(emptyClip => trash.appendChild(emptyClip));
    }

    Array.from(svg.attributes).forEach(attr => {
        const attrName = attr.name.toLowerCase();
        if (!SVG_CLEAN_CONFIG.keepAttrs.has(attrName)) {
            svg.removeAttribute(attr.name);
        }
    });

    if (SVG_CLEAN_CONFIG.removeWhitespaceNodes) {
        removeWhitespaceNodes(svg);
    }

    const serialized = new XMLSerializer().serializeToString(svg);
    return serialized.trim();
}

function removeWhitespaceNodes(node) {
    for (const child of node.childNodes) {
        if (child.nodeType === 3 && /^\s+$/.test(child.textContent)) {
            child.remove();
        } else if (child.nodeType === 1) {
            removeWhitespaceNodes(child);
        }
    }
}