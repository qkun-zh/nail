export const downloadBlobAndNameIt = (blob, file_name_with_suffix) => {
    if (!(blob instanceof Blob) || typeof file_name_with_suffix !== 'string' || !file_name_with_suffix) {
        throw new Error('参数不合法：blob必须是Blob对象，fileNameWithSuffix必须是非空字符串（如"test.pdf"）');
    }

    document.createElement('a').click.call(
        Object.assign(document.createElement('a'), {
            href: URL.createObjectURL(blob),
            download: file_name_with_suffix
        })
    );

    setTimeout(() => URL.revokeObjectURL(blob), 256);
};