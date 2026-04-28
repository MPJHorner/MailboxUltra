// Copy-to-clipboard for code blocks rendered by the markdown builder.
(function () {
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.copy-btn');
    if (!btn) return;
    const block = btn.closest('.code-block');
    if (!block) return;
    const code = block.querySelector('pre code');
    if (!code) return;
    navigator.clipboard.writeText(code.textContent).then(() => {
      const orig = btn.textContent;
      btn.textContent = 'copied';
      btn.classList.add('copied');
      setTimeout(() => {
        btn.textContent = orig;
        btn.classList.remove('copied');
      }, 1200);
    });
  });
})();
