import adapter from '@sveltejs/adapter-static';

const config = {
    kit: {
        adapter: adapter({
            pages: 'build',
            assets: 'build',
            fallback: "index.html",
            precompress: true,
            strict: true
        })
    }
};

export default config;


