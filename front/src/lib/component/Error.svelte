<div class="error-container">
    <span class="letter e">E</span>
    <span class="letter r1">R</span>
    <span class="letter r2">R</span>
    <span class="letter o">O</span>
    <span class="letter r3">R</span>
</div>

<style>
    .error-container {
        display: flex;
        justify-content: center;
        align-items: center;
    }

    .letter {
        color: #000;
        animation: letter-red 2s step-end infinite;
    }

    @keyframes letter-red {
        0%, 20% { color: #000; }
        0.01%, 19.99% { color: #ff0000; }
        20.01%, 39.99% { color: #000; }
        20%, 40% { color: #ff0000; }
        40.01%, 59.99% { color: #000; }
        40%, 60% { color: #ff0000; }
        60.01%, 79.99% { color: #000; }
        60%, 80% { color: #ff0000; }
        80.01%, 99.99% { color: #000; }
        80%, 100% { color: #ff0000; }
    }

    .e { animation-name: letter-red; }
    .r1 { animation-name: letter-red-r1; }
    .r2 { animation-name: letter-red-r2; }
    .o { animation-name: letter-red-o; }
    .r3 { animation-name: letter-red-r3; }

    @keyframes letter-red-r1 {
        0%, 20%, 40%, 60%, 80%, 100% { color: #000; }
        20.01%, 39.99% { color: #ff0000; }
    }

    @keyframes letter-red-r2 {
        0%, 20%, 40%, 60%, 80%, 100% { color: #000; }
        40.01%, 59.99% { color: #ff0000; }
    }

    @keyframes letter-red-o {
        0%, 20%, 40%, 60%, 80%, 100% { color: #000; }
        60.01%, 79.99% { color: #ff0000; }
    }

    @keyframes letter-red-r3 {
        0%, 20%, 40%, 60%, 80%, 100% { color: #000; }
        80.01%, 99.99% { color: #ff0000; }
    }
</style>