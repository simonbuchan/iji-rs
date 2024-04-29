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

      .row {
          display: flex;
          flex-flow: row;
      }

      .col {
          display: flex;
          flex-flow: column;
      }
  `;

  static properties = {
    search: {
      type: String
    },
    state: {}
  };

  constructor() {
    super();
    this.search = "";
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
          <d-tab-set vertical>
            <input
              slot="tab-bar-before"
              type="text"
              placeholder="search"
              .value="${this.search}"
              @input="${this.searchInput}"
            />
            ${Array.from(
              Map.groupBy(
                Object.values(this.state.room.object_instances.values),
                (instance) => this.state.object_types[instance.object_index]
              ).entries()
            )
              .filter(
                ([obj]) =>
                  !this.search ||
                  obj.name.toLowerCase().includes(this.search.toLowerCase())
              )
              .map(([obj, instances]) => {
                return html`
                  <d-tab title="${obj.name} (${instances.length})">
                    <d-tab-set>
                      ${instances.map((instance) =>
                        this.renderInstance(instance)
                      )}
                    </d-tab-set>
                  </d-tab>
                `;
              })}
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

  searchInput(event) {
    this.search = event.target.value;
  }

  renderInstance(instance) {
    let title = `${instance.id}`;
    title += `: ${instance.state.pos[0]},${instance.state.pos[1]}`;

    const sprite =
      instance.state.sprite_asset === null || instance.state.sprite_index < 0
        ? null
        : html`
          <img
            src="/sprite/${instance.state.sprite_index}/${instance.state
              .image_index}"
          />
        `;

    let velocity = instance.state.velocity;
    if (velocity.Cartesian) {
      velocity = `cartesian(${velocity.Cartesian[0]},${velocity.Cartesian[1]})`;
    } else {
      velocity = `polar(${velocity.Polar[0]},${velocity.Polar[1]})`;
    }

    return html`
      <d-tab title="${title}">
        <div class="row">
          ${sprite}
          <div class="col">
            <div>Sprite ${instance.state.sprite_index}</div>
            <div>
              Image ${instance.state.image_index} / Speed
              ${instance.state.image_speed}
            </div>
            <div>
              Blend
              <d-color .value="${instance.state.image_blend_alpha}"></d-color>
            </div>
            <div>Depth ${instance.state.depth}</div>
            <div>Visible ${instance.state.visible}</div>
            <div>Velocity ${velocity}</div>
          </div>
        </div>
        <gml-namespace .value="${instance.vars}"></gml-namespace>
      </d-tab>
    `;
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
          return html`
            <tr>
              <td>${name}</td>
              <td>
                <gml-value .value="${value}">
              </td>
            </tr>
          `;
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
      return JSON.stringify(this.value.String);
    }
    return JSON.stringify(this.value);
  }
}

customElements.define("gml-value", GmlValue);

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
