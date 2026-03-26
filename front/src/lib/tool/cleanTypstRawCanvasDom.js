export function cleanTypstRawCanvasDom(container) {
    if (!container || !(container instanceof HTMLElement)) {return;}

    const targetElements = container.querySelectorAll('.typst-html-semantics');
    const fragment = document.createDocumentFragment();
    for (let i = 0, len = targetElements.length; i < len; i++) {
        const parentElement = targetElements[i].parentElement;
        if (parentElement) {
            fragment.appendChild(parentElement);
        }
    }

    const canvasPageDivs = container.querySelectorAll('div.typst-page.canvas');
    if (canvasPageDivs.length === 0) return;

    canvasPageDivs.forEach((div, index) => {
        div.style.marginBottom = '1rem';

        if (index === canvasPageDivs.length - 1) {

            div.after(document.createElement('br'));
        }
        if (index === 0) {

            div.before(document.createElement('br'));
        }
    });
}