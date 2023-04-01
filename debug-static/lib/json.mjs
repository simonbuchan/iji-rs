import { lit } from "./libs.mjs";

const { LitElement, css, html } = lit;

class Json extends LitElement {
  static styles = css`
    ul {
      padding: 0;
      margin: 0;
    }
    
    th, td {
      text-align: start;
      vertical-align: baseline;
    }

    .null {
      color: coral;
    }

    .boolean {
      color: green;
    }

    .number {
      color: darkblue;
    }

    .string {
      color: brown;
    }
  `;

  static properties = {
    value: Object
  };

  render() {
    switch (typeof this.value) {
      default:
        return html`todo`;
      case "undefined":
        return html`undefined`;
      case "boolean":
      case "number":
      case "string":
        return html`<output class="${typeof this.value}">${this.value}</output>`;
      case "object":
        if (this.value === null) {
          return html`<output class="null">null</output>`;
        }
        if (Array.isArray(this.value)) {
          return html`
            <details>
              <summary>(${this.value.length} items)</summary>
              <ul>
                ${this.value.map((item) => html`
                  <li>
                    <d-json .value="${item}"></d-json>
                  </li>
                `)}
              </ul>
            </details>
          `;
        }
        const entries = Object.entries(this.value);
        if (!entries.length) {
          return html`{}`;
        }
        entries.sort((a, b) => a[0].localeCompare(b[0]));
        return html`
          <details>
            <summary>{ ${entries.length} entries }</summary>
            <table>
              <tbody>
              ${entries.map(([name, value]) => html`
                <tr>
                  <td>${name}</td>
                  <td>
                    <d-json .value="${value}"></d-json>
                  </td>
                </tr>
              `)}
              </tbody>
            </table>
          </details>
        `;
    }
  }
}

customElements.define("d-json", Json);
