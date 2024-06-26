import { LitElement, css, html } from "lit";

// based on https://codepen.io/rpaul/pen/qBWrdOr

function classMap(parts) {
  return Object.entries(parts)
    .filter(([, value]) => value)
    .map(([key]) => key)
    .join(" ");
}

class TabSet extends LitElement {
  static properties = {
    vertical: { type: Boolean }
  };

  static styles = css`
      :host,
      .content {
          display: flex;
          flex-flow: column;
          min-height: 0;
          min-width: 0;
          max-height: max-content;
          flex: 1;
      }

      :host([vertical]) {
          flex-flow: row;
      }

      :host {
          --color-line: #dedede;
          --color-inactive: #f1f1f1;
      }

      :not(:defined) {
          display: none;
      }

      .tab-scroll {
          display: flex;
          flex: none;
          min-height: 0;
          min-width: 0;
      }

      .tab-bar {
          min-width: 0;
          min-height: 0;
          display: flex;
          overflow: auto;
      }

      .tab-scroll.vertical,
      .tab-scroll.vertical > .tab-bar {
          flex-flow: column;
      }

      .content {
          min-width: 0;
          min-height: 0;
          overflow: auto;
          border: 1px solid var(--color-line);
          padding: 15px;
      }

      .tab {
          display: block;
          user-select: none;
          background: var(--color-inactive);
          border: 1px solid var(--color-line);
          padding: 10px 15px;
      }

      .tab-selected {
          background: white;
          border-bottom: 0;
      }
  `;

  getTabs() {
    const slot = this.shadowRoot.querySelector("slot.content");
    return slot ? slot.assignedElements() : [];
  }

  selectTab(selected) {
    for (let tab of this.getTabs()) {
      tab.selected = tab === selected;
    }
    this.requestUpdate();
  }

  firstUpdated() {
    super.firstUpdated();
    const tabs = this.getTabs();
    tabs.find((tab) => tab.selected) || this.selectTab(tabs[0]);
  }

  render() {
    return html`
      <div class="${classMap({ "tab-scroll": true, vertical: this.vertical })}">
        <slot name="tab-bar-before"></slot>
        <nav class="tab-bar">
          ${this.getTabs().map(
            (tab) => html`
              <span
                class="${classMap({ tab: true, "tab-selected": tab.selected })}"
                @click="${() => this.selectTab(tab)}"
              >
                ${tab.title}
              </span>
            `
          )}
        </nav>
        <slot name="tab-bar-after"></slot>
      </div>

      <slot class="content" @slotchange="${() => this.requestUpdate()}"></slot>
    `;
  }
}

class Tab extends LitElement {
  static properties = {
    title: { type: String, reflect: true },
    selected: { type: Boolean, reflect: true }
  };

  static styles = css`
      :host(:not([selected])) {
          display: none;
      }

      :host([selected]) {
          display: contents;
      }
  `;

  render() {
    return html`
      <slot></slot>
    `;
  }
}

customElements.define("d-tab", Tab);
customElements.define("d-tab-set", TabSet);
