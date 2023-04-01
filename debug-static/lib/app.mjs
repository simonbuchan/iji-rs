import { lit } from "./libs.mjs";
import "./json.mjs";
import "./tabs.mjs";

const { LitElement, css, html } = lit;

/** @property {import("./state").Global} state */
class AppRoot extends LitElement {
  static styles = css`
    output {
      display: block;
      font-family: monospace;
      white-space: pre;
    }
  `;

  static properties = {
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
        <button slot="tab-bar-extra" @click="${this.refresh}">refresh</button>
        <d-tab title="globals">
          <gml-namespace .value="${this.state.vars}"></gml-namespace>
        </d-tab>
        <d-tab title="room">
          <d-tab-set vertical>
            ${Object.values(this.state.room.object_instances.values)
              .map((/** @type {import("state").Instance} */ instance) => {
                  const obj = this.state.object_types[instance.object_index];
                  return html`
                    <d-tab title="${obj.name} - ${instance.id}">
                      <img src="/sprite/${instance.state.sprite_index}/${instance.state.image_index}">
                      <gml-namespace .value="${instance.vars}"></gml-namespace>
                    </d-tab>
                  `;
                }
              )}
          </d-tab-set>
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

/** @property {import("./state").Namespace} value */
class GmlNamespace extends LitElement {
  static properties = {
    value: {}
  };

  constructor() {
    super();
    this.value = { vars: {} };
  }

  render() {
    return html`
      <table>
        <thead>
        <tr>
          <td>Name</td>
          <td>Value</td>
        </tr>
        </thead>
        ${Object.entries(this.value.vars).map(([name, value]) => {
          return (
            html`
              <tr>
                <td>${name}</td>
                <td>
                  <gml-value .value="${value}">
                </td>
              </tr>
            `
          );
        })}
      </table>
    `;
  }
}

customElements.define("gml-namespace", GmlNamespace);

/** @property {import("./state").Value} value */
class GmlValue extends LitElement {
  static properties = {
    value: {}
  };

  constructor() {
    super();
    this.value = "Undefined";
  }

  render() {
    if (this.value === "Undefined") {
      return "Undefined";
    }
    if (this.value.Bool !== undefined) {
      return String(this.value.Bool);
    }
    if (this.value.Int !== undefined) {
      return String(this.value.Int);
    }
    if (this.value.Float !== undefined) {
      return this.value.Float.toFixed(1);
    }
    if (this.value.String !== undefined) {
      return `"${this.value.String}"`;
    }
    return JSON.stringify(this.value);
  }
}

customElements.define("gml-value", GmlValue);
