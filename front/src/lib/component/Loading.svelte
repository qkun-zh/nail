<div class="spinner-wrapper">
    <p class="spinner"></p>
</div>

<style>
    .spinner-wrapper {
        margin: 0;
        display: flex;
        justify-content: center;
    }

    .spinner {
        font-size: 24px;
        color: #000;
        border-width: 2px;
        border-style: outset;
        display: inline-block;
        background-color: transparent;
        border-top-color: #fff;
        border-right-color: #000;
        border-bottom-color: #fff;
        border-left-color: #fff;
        animation: spinner-border 1.6s step-end infinite;
        padding: 0 4px;
    }

    .spinner::before {
        content: '-';
        display: inline-block;
        animation: spinner-char 1.6s step-end infinite;
    }

    @keyframes spinner-border {
        0% {
            border-top-color: #fff;
            border-right-color: #000;
            border-bottom-color: #fff;
            border-left-color: #fff;
        }
        12.5% {
            border-top-color: #fff;
            border-right-color: #000;
            border-bottom-color: #000;
            border-left-color: #fff;
        }
        25% {
            border-top-color: #fff;
            border-right-color: #fff;
            border-bottom-color: #000;
            border-left-color: #fff;
        }
        37.5% {
            border-top-color: #fff;
            border-right-color: #fff;
            border-bottom-color: #000;
            border-left-color: #000;
        }
        50% {
            border-top-color: #fff;
            border-right-color: #fff;
            border-bottom-color: #fff;
            border-left-color: #000;
        }
        62.5% {
            border-top-color: #000;
            border-right-color: #fff;
            border-bottom-color: #fff;
            border-left-color: #000;
        }
        75% {
            border-top-color: #000;
            border-right-color: #fff;
            border-bottom-color: #fff;
            border-left-color: #fff;
        }
        87.5% {
            border-top-color: #000;
            border-right-color: #000;
            border-bottom-color: #fff;
            border-left-color: #fff;
        }
        100% {
            border-top-color: #fff;
            border-right-color: #000;
            border-bottom-color: #fff;
            border-left-color: #fff;
        }
    }

    @keyframes spinner-char {
        0% { content: '-'; }
        12.5% { content: '\\'; }
        25% { content: '|'; }
        37.5% { content: '/'; }
        50% { content: '-'; }
        62.5% { content: '\\'; }
        75% { content: '|'; }
        87.5% { content: '/'; }
        100% { content: '-'; }
    }
</style>