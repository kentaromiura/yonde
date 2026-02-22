class ReaderNav extends HTMLElement {
  static observedAttributes = ['has-prev', 'has-next', 'disabled', 'manga'];
  
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._manga = false;
  }
  
  connectedCallback() {
    this._render();
  }
  
  _render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: flex;
          gap: var(--spacing-sm, 8px);
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
          font-size: 18px;
        }
        button:hover:not(:disabled) {
          background: var(--bg-hover, #303030);
          border-color: var(--accent, #4a9eff);
          color: var(--accent, #4a9eff);
        }
        button:disabled {
          opacity: 0.4;
          cursor: not-allowed;
        }
        button:focus-visible {
          outline: 2px solid var(--accent, #4a9eff);
          outline-offset: 2px;
        }
      </style>
      <button id="prevBtn" title="${this._manga ? 'Next page (←)' : 'Previous page (←)'}" aria-label="${this._manga ? 'Next page' : 'Previous page'}">
        ◀
      </button>
      <button id="nextBtn" title="${this._manga ? 'Previous page (→)' : 'Next page (→)'}" aria-label="${this._manga ? 'Previous page' : 'Next page'}">
        ▶
      </button>
    `;
    
    this._prevBtn = this.shadowRoot.getElementById('prevBtn');
    this._nextBtn = this.shadowRoot.getElementById('nextBtn');
    
    this._prevBtn.addEventListener('click', () => this._navigate('prev'));
    this._nextBtn.addEventListener('click', () => this._navigate('next'));
    
    this._updateButtonStates();
    
    this._keydownHandler = (e) => {
      if (this.hasAttribute('disabled')) return;
      if (e.key === 'ArrowLeft') this._navigate('prev');
      if (e.key === 'ArrowRight') this._navigate('next');
    };
    document.addEventListener('keydown', this._keydownHandler);
  }
  
  disconnectedCallback() {
    document.removeEventListener('keydown', this._keydownHandler);
  }
  
  attributeChangedCallback(name, oldVal, newVal) {
    if (name === 'manga') {
      this._manga = newVal !== null;
      if (this._prevBtn) this._render();
    } else {
      this._updateButtonStates();
    }
  }
  
  _updateButtonStates() {
    if (!this._prevBtn || !this._nextBtn) return;
    
    const hasPrev = this.hasAttribute('has-prev');
    const hasNext = this.hasAttribute('has-next');
    const disabled = this.hasAttribute('disabled');
    
    if (this._manga) {
      this._prevBtn.disabled = disabled || !hasNext;
      this._nextBtn.disabled = disabled || !hasPrev;
    } else {
      this._prevBtn.disabled = disabled || !hasPrev;
      this._nextBtn.disabled = disabled || !hasNext;
    }
  }
  
  get hasPrev() { return this.hasAttribute('has-prev'); }
  set hasPrev(val) { this.toggleAttribute('has-prev', val); }
  
  get hasNext() { return this.hasAttribute('has-next'); }
  set hasNext(val) { this.toggleAttribute('has-next', val); }
  
  get disabled() { return this.hasAttribute('disabled'); }
  set disabled(val) { this.toggleAttribute('disabled', val); }
  
  get manga() { return this.hasAttribute('manga'); }
  set manga(val) { this.toggleAttribute('manga', val); }
  
  _navigate(direction) {
    const actualDirection = this._manga 
      ? (direction === 'prev' ? 'next' : 'prev')
      : direction;
    
    const btn = direction === 'prev' ? this._prevBtn : this._nextBtn;
    if (btn.disabled) return;
    
    this.dispatchEvent(new CustomEvent('navigate', {
      detail: { direction: actualDirection },
      bubbles: true
    }));
  }
}

customElements.define('reader-nav', ReaderNav);
