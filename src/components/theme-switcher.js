class ThemeSwitcher extends HTMLElement {
  static observedAttributes = ['theme'];
  
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._theme = this._loadTheme();
  }
  
  connectedCallback() {
    this._applyTheme(this._theme);
    this._render();
  }
  
  _render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: inline-block;
        }
        button {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 40px;
          height: 40px;
          border: 1px solid var(--border, #3a3a3a);
          border-radius: var(--radius-md, 8px);
          background: var(--bg-elevated, #252525);
          color: var(--fg, #e8e8e8);
          cursor: pointer;
          transition: all 150ms ease;
          font-size: 20px;
        }
        button:hover {
          background: var(--bg-hover, #303030);
          border-color: var(--accent, #4a9eff);
        }
        button:focus-visible {
          outline: 2px solid var(--accent, #4a9eff);
          outline-offset: 2px;
        }
        .icon-sun { display: none; }
        .icon-moon { display: block; }
        :host([theme="light"]) .icon-sun { display: block; }
        :host([theme="light"]) .icon-moon { display: none; }
      </style>
      <button id="toggle" title="Toggle theme" aria-label="Toggle theme">
        <span class="icon-sun">☀️</span>
        <span class="icon-moon">🌙</span>
      </button>
    `;
    
    this._toggleBtn = this.shadowRoot.getElementById('toggle');
    this._toggleBtn.addEventListener('click', () => this.toggle());
    this.setAttribute('theme', this._theme);
  }
  
  get theme() { return this._theme; }
  set theme(val) {
    if (this._theme === val) return;
    this._theme = val;
    this._applyTheme(val);
    this._saveTheme(val);
    this.setAttribute('theme', val);
    this.dispatchEvent(new CustomEvent('theme-change', {
      detail: { theme: this._theme },
      bubbles: true
    }));
  }
  
  toggle() {
    this.theme = this._theme === 'dark' ? 'light' : 'dark';
  }
  
  _applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
  }
  
  _saveTheme(theme) {
    try {
      localStorage.setItem('reader-theme', theme);
    } catch (e) {}
  }
  
  _loadTheme() {
    try {
      return localStorage.getItem('reader-theme') || 'dark';
    } catch (e) {
      return 'dark';
    }
  }
}

customElements.define('theme-switcher', ThemeSwitcher);
