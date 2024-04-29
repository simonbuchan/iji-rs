export interface Global {
  assets: Assets;
  consts: Namespace;
  last_instance_id: number;
  object_types: Record<number, ObjectAsset>;
  room: Room;
  vars: Namespace;
}

export interface Namespace {
  vars: Record<string, Value>;
}

export type Value =
  | "Undefined"
  | { Bool: boolean }
  | { Int: number }
  | { Float: number }
  | { String: string };

export interface Assets {
  backgrounds: AssetMap<Background>;
  sprites: AssetMap<Sprite>;
}

export interface AssetMap<T> {
  indices: Record<string, number>;
  items: Record<number, [string, T]>;
}

export interface Background {
  tile_enabled: boolean;
}

export interface Sprite {}

export interface ObjectAsset {
  name: string;
  object: ObjectType;
  parent_index: number | null;
}

export interface ObjectType {
  instances: number[];
}

export interface Room {
  elapsed: number;
  speed: number;
  background_layers: Layer[];
  foreground_layers: Layer[];
  object_instances: DoubleMap<Instance>;
  script_instances: Record<number, null | Value[]>;
  tiles: Tile[];
  view: View;
}

export interface Tile {
  depth: number;
  asset: number;
  pos: Vec2;
  source: Rect;
}

export interface View {
  offset: Vec2;
  size: Vec2;
}

export interface DoubleMap<T> {
  names: Record<string, number>;
  values: Record<number, T>;
}

export interface Layer {
  enabled: boolean;
  pos: Vec2;
  tile: boolean;
  asset: number;
}

export interface Instance {
  id: number;
  object_index: number;
  parent_object_index: null | number;
  depth: number;
  vars: Namespace;
  state: InstanceState;
}

export interface InstanceState {
  pos: Vec2;
  visible: boolean;
  sprite_asset: number | null;
  sprite_index: number;
  image_speed: number;
  image_index: number;
  image_blend_alpha: Color;
  velocity: Velocity;
}

export type Vec2 = [number, number];

export interface Rect {
  point: Vec2;
  size: Vec2;
}

export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
}

export type Velocity =
  | { Cartesian: Vec2; Polar?: never }
  | { Cartesian?: never; Polar: Vec2 };
