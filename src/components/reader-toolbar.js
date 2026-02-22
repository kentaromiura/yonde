class ReaderToolbar extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }
  
  connectedCallback() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: flex;
          align-items: center;
          gap: var(--spacing-md, 16px);
          padding: var(--spacing-sm, 8px) var(--spacing-md, 16px);
          background: var(--bg-elevated, #252525);
          border-bottom: 1px solid var(--border, #3a3a3a);
          box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.3));
        }
        :host([position="bottom"]) {
          border-bottom: none;
          border-top: 1px solid var(--border, #3a3a3a);
        }
        .spacer {
          flex: 1;
        }
        ::slotted(*) {
          flex-shrink: 0;
        }
      </style>
      <slot name="start"></slot>
      <div class="spacer"></div>
      <slot></slot>
      <div class="spacer"></div>
      <slot name="end"></slot>
    `;
  }
}

customElements.define('reader-toolbar', ReaderToolbar);
