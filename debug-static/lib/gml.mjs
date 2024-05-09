import { css, LitElement, html } from "lit";
import "@lit-labs/virtualizer";

/** @property {import("./state").Namespace} value */
class GmlNamespace extends LitElement {
  static styles = css`
    :host { flex: 1; display: flex; flex-flow: column }
    .header { border-bottom: solid 1px #ccc }
    .body { flex: 1 }
    .header, .row { display: flex; flex-flow: row; gap: 10px; height: 20px }
    .row { width: 100% }
    label {
      display: block;
      width: 150px;
      font-family: monospace;
      white-space: pre;
   }
    gml-value { flex: 1; overflow: hidden; text-overflow: ellipsis }
  `;

  static properties = {
    value: {},
    filter: { type: String }
  };

  constructor() {
    super();
    this.value = { vars: {} };
    this.filter = "";
  }

  render() {
    let items = Object.entries(this.value.vars);
    if (this.filter) {
      items = items.filter(([name]) => name.includes(this.filter));
    }
    items.sort(([a], [b]) => a.localeCompare(b));

    return html`
      <div class="header">
        <label>Name</label>
        <span>Value</span>
        <input
          type="search"
          placeholder="filter names"
          .value="${this.filter}"
          @input="${this.filterInput}"
        >
      </div>
      <lit-virtualizer
        class="body"
        scroller
        .items="${items}"
        .layout="${{
          direction: "vertical"
        }}"
        .renderItem="${([name, value]) => html`
          <div class="row">
            <label>${name}</label>
            <gml-value .value="${value}">
          </div>
        `}"
      ></lit-virtualizer>
    `;
  }

  filterInput(event) {
    this.filter = event.target.value;
  }
}

customElements.define("gml-namespace", GmlNamespace);

/** @property {import("./state").Value} value */
class GmlValue extends LitElement {
  static styles = css`
    output {
      font-family: monospace;
      white-space: pre;
    }
    .undefined { color: gray; }
    .null { color: coral; }
    .boolean { color: green; }
    .int { color: darkblue; }
    .float { color: purple; }
    .string { color: brown; }
  `;
  static properties = {
    value: {}
  };

  constructor() {
    super();
    this.value = "Undefined";
  }

  render() {
    if (this.value === "Undefined") {
      return html`
        <output class="undefined">Undefined</output>`;
    }
    if (this.value.Bool !== undefined) {
      return html`
        <output class="bool">${this.value.Bool}</output>`;
    }
    if (this.value.Int !== undefined) {
      return html`
        <output class="int">${this.value.Int}</output>`;
    }
    if (this.value.Float !== undefined) {
      return html`
        <output class="float">${this.value.Float.toFixed(1)}</output>`;
    }
    if (this.value.String !== undefined) {
      return html`
        <output class="string">${JSON.stringify(this.value.String)}</output>`;
    }
    return JSON.stringify(this.value);
  }
}

customElements.define("gml-value", GmlValue);
