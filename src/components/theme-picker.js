class ThemePicker extends HTMLElement {
  static observedAttributes = ['current-theme'];
  
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._themes = this._loadThemes();
    this._currentTheme = this._loadCurrentTheme();
    this._editingId = null;
    this._previewVariables = null;
  }
  
  connectedCallback() {
    this._applyTheme(this._currentTheme);
    this._render();
  }
  
  _render() {
    const builtinThemes = [
      { id: 'dark', name: 'Dark', builtin: true },
      { id: 'light', name: 'Light', builtin: true }
    ];
    
    const allThemes = [...builtinThemes, ...this._themes];
    
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: inline-block;
          position: relative;
        }
        .trigger {
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
        .trigger:hover {
          background: var(--bg-hover, #303030);
          border-color: var(--accent, #4a9eff);
        }
        .trigger:focus-visible {
          outline: 2px solid var(--accent, #4a9eff);
          outline-offset: 2px;
        }
        .dropdown {
          position: absolute;
          top: calc(100% + 4px);
          right: 0;
          min-width: 180px;
          background: var(--bg-elevated, #252525);
          border: 1px solid var(--border, #3a3a3a);
          border-radius: var(--radius-md, 8px);
          box-shadow: var(--shadow-lg, 0 10px 15px rgba(0,0,0,0.5));
          z-index: 1000;
          display: none;
        }
        .dropdown.open {
          display: block;
        }
        .theme-list {
          list-style: none;
          padding: 4px;
          margin: 0;
        }
        .theme-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          border-radius: 4px;
          cursor: pointer;
          transition: background 150ms ease;
          font-size: 14px;
        }
        .theme-item:hover {
          background: var(--bg-hover, #303030);
        }
        .theme-item.active {
          background: var(--accent, #4a9eff);
          color: var(--bg, #1a1a1a);
        }
        .theme-actions {
          display: flex;
          gap: 4px;
          align-items: center;
        }
        .theme-item .action-btn {
          background: none;
          border: none;
          color: inherit;
          cursor: pointer;
          padding: 2px 4px;
          opacity: 0.6;
          font-size: 12px;
          line-height: 1;
        }
        .theme-item .action-btn:hover {
          opacity: 1;
        }
        .theme-item.active .action-btn:hover {
          color: var(--error, #ef4444);
        }
        .divider {
          height: 1px;
          background: var(--border, #3a3a3a);
          margin: 4px 0;
        }
        .add-theme-btn {
          display: flex;
          align-items: center;
          gap: 8px;
          width: 100%;
          padding: 8px 12px;
          border: none;
          background: none;
          color: var(--fg, #e8e8e8);
          cursor: pointer;
          font-size: 14px;
        }
        .add-theme-btn:hover {
          background: var(--bg-hover, #303030);
          border-radius: 4px;
        }
        .modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.6);
          display: none;
          align-items: center;
          justify-content: center;
          z-index: 10000;
        }
        .modal-overlay.open {
          display: flex;
        }
        .modal {
          background: var(--bg-elevated, #252525);
          border: 1px solid var(--border, #3a3a3a);
          border-radius: 12px;
          padding: 20px;
          min-width: 340px;
          max-width: 90vw;
        }
        .modal h3 {
          margin: 0 0 16px;
          color: var(--fg, #e8e8e8);
          font-size: 16px;
        }
        .form-group {
          margin-bottom: 16px;
        }
        .form-group label {
          display: block;
          margin-bottom: 4px;
          color: var(--fg-muted, #888888);
          font-size: 12px;
        }
        .form-group input[type="text"] {
          width: 100%;
          padding: 8px;
          border: 1px solid var(--border, #3a3a3a);
          border-radius: 4px;
          background: var(--bg, #1a1a1a);
          color: var(--fg, #e8e8e8);
          font-size: 14px;
          box-sizing: border-box;
        }
        .color-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 12px;
        }
        .color-input label {
          display: block;
          margin-bottom: 4px;
          color: var(--fg-muted, #888888);
          font-size: 11px;
        }
        .color-input input[type="color"] {
          width: 100%;
          height: 32px;
          padding: 2px;
          border: 1px solid var(--border, #3a3a3a);
          border-radius: 4px;
          cursor: pointer;
          background: var(--bg, #1a1a1a);
        }
        .modal-actions {
          display: flex;
          gap: 8px;
          justify-content: flex-end;
          margin-top: 20px;
        }
        .btn {
          padding: 8px 16px;
          border: 1px solid var(--border, #3a3a3a);
          border-radius: 4px;
          background: var(--bg-elevated, #252525);
          color: var(--fg, #e8e8e8);
          cursor: pointer;
          font-size: 14px;
        }
        .btn:hover {
          background: var(--bg-hover, #303030);
        }
        .btn-primary {
          background: var(--accent, #4a9eff);
          border-color: var(--accent, #4a9eff);
          color: var(--bg, #1a1a1a);
        }
        .btn-primary:hover {
          filter: brightness(1.1);
        }
      </style>
      <button class="trigger" id="trigger" title="Theme">🎨</button>
      <div class="dropdown" id="dropdown">
        <ul class="theme-list">
          ${allThemes.map(t => `
            <li class="theme-item ${t.id === this._currentTheme ? 'active' : ''}" data-id="${t.id}">
              <span>${t.name}</span>
              <span class="theme-actions">
                ${!t.builtin ? '<button class="action-btn edit-btn" data-edit="' + t.id + '" title="Edit">✎</button>' : ''}
                ${!t.builtin ? '<button class="action-btn delete-btn" data-delete="' + t.id + '" title="Delete">✕</button>' : ''}
                ${t.id === this._currentTheme ? '<span>✓</span>' : ''}
              </span>
            </li>
          `).join('')}
        </ul>
        <div class="divider"></div>
        <button class="add-theme-btn" id="addBtn">+ New Theme</button>
      </div>
      <div class="modal-overlay" id="modal">
        <div class="modal">
          <h3 id="modalTitle">Create Theme</h3>
          <div class="form-group">
            <label>Theme Name</label>
            <input type="text" id="themeName" placeholder="My Theme">
          </div>
          <div class="form-group">
            <label>Colors (live preview)</label>
            <div class="color-grid">
              <div class="color-input">
                <label>Background</label>
                <input type="color" id="colorBg" value="#1a1a1a">
              </div>
              <div class="color-input">
                <label>Foreground</label>
                <input type="color" id="colorFg" value="#e8e8e8">
              </div>
              <div class="color-input">
                <label>Accent</label>
                <input type="color" id="colorAccent" value="#4a9eff">
              </div>
              <div class="color-input">
                <label>Elevated</label>
                <input type="color" id="colorElevated" value="#252525">
              </div>
              <div class="color-input">
                <label>Muted</label>
                <input type="color" id="colorMuted" value="#888888">
              </div>
              <div class="color-input">
                <label>Border</label>
                <input type="color" id="colorBorder" value="#3a3a3a">
              </div>
            </div>
          </div>
          <div class="modal-actions">
            <button class="btn" id="cancelBtn">Cancel</button>
            <button class="btn btn-primary" id="saveBtn">Save</button>
          </div>
        </div>
      </div>
    `;
    
    this._setupEvents();
  }
  
  _setupEvents() {
    const trigger = this.shadowRoot.getElementById('trigger');
    const dropdown = this.shadowRoot.getElementById('dropdown');
    const addBtn = this.shadowRoot.getElementById('addBtn');
    const modal = this.shadowRoot.getElementById('modal');
    const cancelBtn = this.shadowRoot.getElementById('cancelBtn');
    const saveBtn = this.shadowRoot.getElementById('saveBtn');
    const themeNameInput = this.shadowRoot.getElementById('themeName');
    
    trigger.addEventListener('click', (e) => {
      e.stopPropagation();
      dropdown.classList.toggle('open');
    });
    
    document.addEventListener('click', (e) => {
      if (!this.contains(e.target)) {
        dropdown.classList.remove('open');
      }
    });
    
    dropdown.addEventListener('click', (e) => {
      const editBtn = e.target.closest('.edit-btn');
      const deleteBtn = e.target.closest('.delete-btn');
      const themeItem = e.target.closest('.theme-item');
      
      if (deleteBtn) {
        e.stopPropagation();
        e.preventDefault();
        const id = deleteBtn.dataset.delete;
        this._deleteTheme(id);
        return;
      }
      
      if (editBtn) {
        e.stopPropagation();
        e.preventDefault();
        const id = editBtn.dataset.edit;
        this._openEditModal(id);
        dropdown.classList.remove('open');
        return;
      }
      
      if (themeItem && !e.target.closest('.theme-actions')) {
        this._selectTheme(themeItem.dataset.id);
        dropdown.classList.remove('open');
      }
    });
    
    addBtn.addEventListener('click', () => {
      this._editingId = null;
      this._previewVariables = null;
      this._resetModalForm();
      this.shadowRoot.getElementById('modalTitle').textContent = 'Create Theme';
      dropdown.classList.remove('open');
      modal.classList.add('open');
    });
    
    cancelBtn.addEventListener('click', () => {
      this._cancelPreview();
      modal.classList.remove('open');
    });
    
    modal.addEventListener('click', (e) => {
      if (e.target === modal) {
        this._cancelPreview();
        modal.classList.remove('open');
      }
    });
    
    saveBtn.addEventListener('click', () => {
      this._saveTheme();
      modal.classList.remove('open');
    });
    
    const colorInputs = ['colorBg', 'colorFg', 'colorAccent', 'colorElevated', 'colorMuted', 'colorBorder'];
    colorInputs.forEach(id => {
      const input = this.shadowRoot.getElementById(id);
      if (input) {
        input.addEventListener('input', () => this._previewColors());
      }
    });
  }
  
  _resetModalForm() {
    const defaults = {
      themeName: '',
      colorBg: '#1a1a1a',
      colorFg: '#e8e8e8',
      colorAccent: '#4a9eff',
      colorElevated: '#252525',
      colorMuted: '#888888',
      colorBorder: '#3a3a3a'
    };
    
    Object.entries(defaults).forEach(([id, value]) => {
      const el = this.shadowRoot.getElementById(id);
      if (el) el.value = value;
    });
  }
  
  _openEditModal(id) {
    const theme = this._themes.find(t => t.id === id);
    if (!theme) return;
    
    this._editingId = id;
    this._previewVariables = null;
    
    this.shadowRoot.getElementById('modalTitle').textContent = 'Edit Theme';
    this.shadowRoot.getElementById('themeName').value = theme.name;
    this.shadowRoot.getElementById('colorBg').value = theme.variables['--bg'] || '#1a1a1a';
    this.shadowRoot.getElementById('colorFg').value = theme.variables['--fg'] || '#e8e8e8';
    this.shadowRoot.getElementById('colorAccent').value = theme.variables['--accent'] || '#4a9eff';
    this.shadowRoot.getElementById('colorElevated').value = theme.variables['--bg-elevated'] || '#252525';
    this.shadowRoot.getElementById('colorMuted').value = theme.variables['--fg-muted'] || '#888888';
    this.shadowRoot.getElementById('colorBorder').value = theme.variables['--border'] || '#3a3a3a';
    
    this.shadowRoot.getElementById('modal').classList.add('open');
  }
  
  _previewColors() {
    const variables = this._getFormVariables();
    this._previewVariables = variables;
    this._applyVariables(variables);
  }
  
  _cancelPreview() {
    if (this._previewVariables) {
      this._applyTheme(this._currentTheme);
      this._previewVariables = null;
    }
  }
  
  _getFormVariables() {
    return {
      '--bg': this.shadowRoot.getElementById('colorBg').value,
      '--fg': this.shadowRoot.getElementById('colorFg').value,
      '--accent': this.shadowRoot.getElementById('colorAccent').value,
      '--bg-elevated': this.shadowRoot.getElementById('colorElevated').value,
      '--fg-muted': this.shadowRoot.getElementById('colorMuted').value,
      '--border': this.shadowRoot.getElementById('colorBorder').value,
      '--bg-hover': this._lightenColor(this.shadowRoot.getElementById('colorElevated').value, 10)
    };
  }
  
  _saveTheme() {
    const name = this.shadowRoot.getElementById('themeName').value.trim() || 'Custom Theme';
    const variables = this._getFormVariables();
    
    if (this._editingId) {
      const theme = this._themes.find(t => t.id === this._editingId);
      if (theme) {
        theme.name = name;
        theme.variables = variables;
      }
      this._saveThemes();
      this._previewVariables = null;
      if (this._currentTheme === this._editingId) {
        this._applyVariables(variables);
      }
      this._render();
    } else {
      const id = 'custom-' + Date.now();
      const theme = { id, name, variables };
      this._themes.push(theme);
      this._saveThemes();
      this._previewVariables = null;
      this._selectTheme(id);
    }
    
    this._editingId = null;
  }
  
  _deleteTheme(id) {
    const index = this._themes.findIndex(t => t.id === id);
    if (index === -1) return;
    
    this._themes.splice(index, 1);
    this._saveThemes();
    
    if (this._currentTheme === id) {
      this._selectTheme('dark');
    } else {
      this._render();
    }
  }
  
  _getThemeName(id) {
    if (id === 'dark') return 'Dark';
    if (id === 'light') return 'Light';
    const theme = this._themes.find(t => t.id === id);
    return theme ? theme.name : id;
  }
  
  _selectTheme(id) {
    this._currentTheme = id;
    this._applyTheme(id);
    this._saveCurrentTheme(id);
    this._render();
    this.dispatchEvent(new CustomEvent('theme-change', {
      detail: { theme: id },
      bubbles: true
    }));
  }
  
  _applyTheme(id) {
    const themes = {
      dark: {
        '--bg': '#1a1a1a',
        '--bg-elevated': '#252525',
        '--bg-hover': '#303030',
        '--fg': '#e8e8e8',
        '--fg-muted': '#888888',
        '--accent': '#4a9eff',
        '--border': '#3a3a3a'
      },
      light: {
        '--bg': '#ffffff',
        '--bg-elevated': '#f5f5f5',
        '--bg-hover': '#e8e8e8',
        '--fg': '#1a1a1a',
        '--fg-muted': '#666666',
        '--accent': '#4a9eff',
        '--border': '#d0d0d0'
      }
    };
    
    if (themes[id]) {
      document.documentElement.setAttribute('data-theme', id);
      this._applyVariables(themes[id]);
    } else {
      const theme = this._themes.find(t => t.id === id);
      if (theme) {
        document.documentElement.setAttribute('data-theme', id);
        this._applyVariables(theme.variables);
      }
    }
  }
  
  _applyVariables(vars) {
    Object.entries(vars).forEach(([key, value]) => {
      document.documentElement.style.setProperty(key, value);
    });
  }
  
  _lightenColor(hex, percent) {
    const num = parseInt(hex.slice(1), 16);
    const r = Math.min(255, (num >> 16) + Math.round(255 * percent / 100));
    const g = Math.min(255, ((num >> 8) & 0x00FF) + Math.round(255 * percent / 100));
    const b = Math.min(255, (num & 0x0000FF) + Math.round(255 * percent / 100));
    return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
  }
  
  _loadThemes() {
    try {
      return JSON.parse(localStorage.getItem('reader-custom-themes')) || [];
    } catch (e) {
      return [];
    }
  }
  
  _saveThemes() {
    try {
      localStorage.setItem('reader-custom-themes', JSON.stringify(this._themes));
    } catch (e) {}
  }
  
  _loadCurrentTheme() {
    try {
      return localStorage.getItem('reader-current-theme') || 'dark';
    } catch (e) {
      return 'dark';
    }
  }
  
  _saveCurrentTheme(id) {
    try {
      localStorage.setItem('reader-current-theme', id);
    } catch (e) {}
  }
  
  get theme() { return this._currentTheme; }
  set theme(id) { this._selectTheme(id); }
}

customElements.define('theme-picker', ThemePicker);
