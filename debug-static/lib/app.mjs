import { LitElement, css, html } from "lit";

import "./json.mjs";
import "./tabs.mjs";
import "./gml.mjs";
import "./tab/room.mjs";

/** @property {import("./state").Global} state */
class AppRoot extends LitElement {
  static styles = css`
    :host { flex: 1 }
    
    button {
      font: inherit;
      align-self: center;
      padding: 0.5em 1em;
      border: 1px solid #ccc;
      background: none;
      cursor: pointer;
      
      &:hover { background: #f1f1f1 }
    }

    output {
      display: block;
      font-family: monospace;
      white-space: pre;
    }
    
    gml-namespace { flex: 1 }
  `;

  static properties = {
    search: {
      type: String
    },
    state: {}
  };

  constructor() {
    super();
    void this.refresh();
  }

  render() {
    if (!this.state) {
      return html`loading...`;
    }
    return html`
      <d-tab-set>
        <button slot="tab-bar-after" @click="${this.refresh}">refresh</button>
        <d-tab title="globals">
          <gml-namespace .value="${this.state.vars}"></gml-namespace>
        </d-tab>
        <d-tab title="room">
          <d-room .state="${this.state}"></d-room>
        </d-tab>
        <d-tab title="raw">
          <d-json .value="${this.state}"></d-json>
        </d-tab>
      </d-tab-set>
    `;
  }

  async refresh() {
    const res = await fetch("/state");
    this.state = await res.json();
  }

}

customElements.define("app-root", AppRoot);

/** @property {import("./state").Color} value */
class DColor extends LitElement {
  static properties = {
    value: {}
  };

  constructor() {
    super();
    this.value = { r: 0, g: 0, b: 0, a: 1 };
  }

  render() {
    const { r, g, b, a } = this.value;
    const background = `rgb(${r * 255} ${g * 255} ${b * 255})`;
    let dark = r + g + b < 0.5 * 3;
    const color = dark ? "white" : "black";

    return html`
      <output style="background: ${background}; color: ${color}; border: 1px solid ${color}">
        ${r} ${g} ${b} ${a}
      </output>
    `;
  }
}

customElements.define("d-color", DColor);
