import { css, html, LitElement } from "lit";

/** @property {import("./state").Global} state */
class Room extends LitElement {
  static styles = css`
    :host { flex: 1; display: flex; flex-flow: column }
    input { font: inherit }
    .row { display: flex; flex-flow: row }
    .col { display: flex; flex-flow: column }
    img { max-width: 200px; max-height: 200px };
  `;

  static properties = {
    filter: { type: String },
    state: {}
  };

  constructor() {
    super();
    this.filter = "";
  }

  render() {
    if (!this.state) {
      return html``;
    }
    return html`
      <d-tab-set vertical>
        <input
          slot="tab-bar-before"
          type="text"
          placeholder="filter instances"
          .value="${this.filter}"
          @input="${this.filterInput}"
        />
        ${Array.from(
          Map.groupBy(
            Object.values(this.state.room.object_instances.values),
            (instance) => this.state.object_types[instance.object_index]
          ).entries()
        )
          .filter(
            ([obj]) => !this.filter || obj.name.includes(this.search)
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
    `;
  }

  filterInput(event) {
    this.filter = event.target.value;
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

customElements.define("d-room", Room);
