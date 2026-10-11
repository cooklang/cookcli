// Says why a change was turned away when the server's sign-in check refused
// it, which it marks with `X-Cook-Refused`. The page was drawn for someone
// signed in whose sign-in has since ended (`sign-in`: offer to sign in and come
// back here) or whose role was lowered (`role`: signing in again won't help).
// Each page still shows its own error; this adds what to do about it, for
// every write, the ones a page reverts without a word (a shopping list tick)
// included.
(function () {
    const send = window.fetch;
    window.fetch = function (resource, options) {
        const method = String((options && options.method)
            || (resource instanceof Request ? resource.method : 'GET')).toUpperCase();
        const response = send.apply(this, arguments);
        if (method === 'GET' || method === 'HEAD') return response;
        return response.then(answer => {
            const refused = answer.headers.get('X-Cook-Refused');
            if (refused) show(refused);
            return answer;
        });
    };

    function show(reason) {
        const notice = document.getElementById('refused-notice');
        if (!notice) return;
        const signIn = reason === 'sign-in';
        notice.querySelector('[data-refused-text]').textContent =
            signIn ? notice.dataset.signIn : notice.dataset.role;
        notice.querySelector('[data-refused-sign-in]').classList.toggle('hidden', !signIn);
        notice.classList.remove('hidden');
    }

    document.addEventListener('click', event => {
        if (!event.target.closest('[data-refused-close]')) return;
        document.getElementById('refused-notice').classList.add('hidden');
    });
})();
