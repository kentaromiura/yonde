class ReaderIndicator extends HTMLElement {
  static observedAttributes = ['current', 'total'];
  
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }
  
  connectedCallback() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: inline-flex;
          align-items: center;
          gap: var(--spacing-xs, 4px);
          padding: var(--spacing-sm, 8px) var(--spacing-md, 16px);
          background: var(--bg-elevated, #252525);
          border: 1px solid var(--border, #3a3a3a);
          border-radius: var(--radius-md, 8px);
          font-size: var(--font-size-md, 14px);
          color: var(--fg, #e8e8e8);
        }
        .current {
          color: var(--accent, #4a9eff);
          font-weight: 600;
          min-width: 2ch;
          text-align: right;
        }
        .separator {
          color: var(--fg-muted, #888888);
        }
        .total {
          color: var(--fg-muted, #888888);
          min-width: 2ch;
        }
      </style>
      <span class="current" id="current">1</span>
      <span class="separator">/</span>
      <span class="total" id="total">0</span>
    `;
    
    this._currentEl = this.shadowRoot.getElementById('current');
    this._totalEl = this.shadowRoot.getElementById('total');
  }
  
  attributeChangedCallback(name, oldVal, newVal) {
    if (!this._currentEl || !this._totalEl) return;
    
    if (name === 'current') {
      this._currentEl.textContent = newVal || '1';
    }
    if (name === 'total') {
      this._totalEl.textContent = newVal || '0';
    }
  }
  
  get current() { return parseInt(this.getAttribute('current')) || 1; }
  set current(val) { this.setAttribute('current', val); }
  
  get total() { return parseInt(this.getAttribute('total')) || 0; }
  set total(val) { this.setAttribute('total', val); }
}

customElements.define('reader-indicator', ReaderIndicator);
