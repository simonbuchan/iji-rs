//! Based on https://github.com/IsmAvatar/LateralGM
//! and https://enigma-dev.org/docs/Wiki/GM_format

use nom::bytes::complete::take;
use nom::combinator::flat_map;
use nom::error::ParseError;
use nom::number::complete::le_u32;
use nom_derive::{NomLE, Parse};
use std::io::Write;

use settings::GameSettings;

mod settings;

pub fn parse() -> Content {
    let mut data = std::fs::read("ref/iji.gmk").unwrap();
    let (header, start) = parse_offset::<FileHeader>(&data, 0);

    // print!("generating decode table from seed {}...", header.crypt.seed);
    std::io::stdout().flush().unwrap();
    let decode_table = generate_decode_table(header.crypt.seed);
    // println!("done");

    // print!("decoding...");
    std::io::stdout().flush().unwrap();
    for pos in (start + 1)..data.len() {
        data[pos] = decode_table[data[pos] as usize].wrapping_sub((pos % 256) as u8);
    }
    // println!("done");

    // reborrow after mutation
    let (content, _parsed) = parse_offset::<Content>(&data, start);
    // dbg!(content, parsed, data.len() - parsed);
    content
}

fn parse_offset<'nom, T: Parse<&'nom [u8], nom::error::VerboseError<&'nom [u8]>>>(
    input: &'nom [u8],
    offset: usize,
) -> (T, usize) {
    match T::parse(&input[offset..]) {
        Ok((remaining, data)) => (data, input.len() - remaining.len()),
        Err(nom::Err::Failure(error) | nom::Err::Error(error)) => {
            let mut messages = String::new();
            for e in error.errors {
                std::fmt::Write::write_fmt(
                    &mut messages,
                    format_args!("- at {}: {:?}\n", input.len() - e.0.len(), e.1),
                )
                .unwrap();
            }
            panic!("Parse failed:\n{messages}");
        }
        Err(nom::Err::Incomplete(needed)) => {
            panic!("incomplete: {needed:?}");
        }
    }
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
struct FileHeader {
    _magic: GmkMagic,
    _ver: u32,
    crypt: GMKrypt,
}

#[repr(u32)]
#[derive(Debug, Eq, PartialEq, NomLE)]
#[nom(GenericErrors)]
enum GmkMagic {
    Valid = 1234321,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
struct GMKrypt {
    _s1: u32,
    #[nom(SkipAfter = "(_s1 * 4)")]
    _s2: u32,
    #[nom(SkipAfter = "(_s2 * 4)")]
    seed: u32,
}

fn generate_decode_table(seed: u32) -> [u8; 256] {
    let a = 6 + seed % 250;
    let b = seed / 250;
    let mut encode_table = [0u8; 256];
    for i in 0..=255u8 {
        encode_table[i as usize] = i;
    }
    for i in 1..10001 {
        let j = (1 + (i * a + b) % 254) as usize;
        encode_table.swap(j, j + 1);
    }

    let mut table = [0u8; 256];
    for i in 1..=255u8 {
        table[encode_table[i as usize] as usize] = i;
    }
    table
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Content {
    pub game_id: u32,
    pub game_guid: Guid,
    pub settings: GameSettings,
    // triggers: Chunk<'input>,
    // constants: Chunk<'input>,
    pub sounds: ResourceChunk<Sound>,
    pub sprites: ResourceChunk<Sprite>,
    pub backgrounds: ResourceChunk<Background>,
    pub paths: ResourceChunk<Path>,
    pub scripts: ResourceChunk<Script>,
    pub fonts: ResourceChunk<Font>,
    pub timelines: ResourceChunk<Timeline>,
    pub objects: ResourceChunk<Object>,
    pub rooms: ResourceChunk<Room>,
    pub last_instance_id: u32,
    pub last_tile_id: u32,
    pub includes: Chunk<Include>,
    pub extensions: Chunk<String32>,
    pub information: GameInformation,
    pub library_creation_codes: Chunk<String32>,
    pub room_order: Chunk<u32>,
    #[nom(Count = "12")]
    pub resource_tree: Vec<ResourceTreeItem>,
}

// #[derive(Debug)]
// struct DebugChunk<T> {
//     ver: u32,
//     count: u32,
//     first_item: Option<NamedResourceItem<T>>,
// }
//
// impl<'nom, T: Parse<&'nom [u8], E>, E: ParseError<&'nom [u8]>> Parse<&'nom [u8], E>
//     for DebugChunk<T>
// {
//     fn parse(i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self, E> {
//         let (i, ver) = le_u32(i)?;
//         let (mut i, count) = le_u32(i)?;
//         dbg!(ver, count);
//         let mut first_item = None;
//         for ix in 0..count {
//             let (i2, present) = Bool32::parse(i)?;
//             i = i2;
//             if present == Bool32::True {
//                 dbg!(ix);
//                 let (i2, item) = NamedResourceItem::<T>::parse(i)?;
//                 i = i2;
//                 first_item = Some(item);
//                 break;
//             }
//         }
//         Ok((
//             i,
//             Self {
//                 ver,
//                 count,
//                 first_item,
//             },
//         ))
//     }
// }

#[derive(NomLE)]
#[nom(GenericErrors)]
pub struct ResourceChunk<T> {
    pub ver: u32,
    #[nom(LengthCount = "le_u32", Parse = "parse_cond32")]
    pub items: Vec<Option<ResourceItem<T>>>,
}

impl<'a, T> IntoIterator for &'a ResourceChunk<T> {
    type Item = (&'a str, &'a T);
    type IntoIter = ResourceChunkIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ResourceChunkIter(self.items.iter().flatten())
    }
}

pub struct ResourceChunkIter<'a, T>(
    std::iter::Flatten<std::slice::Iter<'a, Option<ResourceItem<T>>>>,
);

impl<'a, T> Iterator for ResourceChunkIter<'a, T> {
    type Item = (&'a str, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.0.next()?;
        Some((&i.name.0, &i.data))
    }
}

impl<T> ResourceChunk<T> {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.into_iter()
    }

    pub fn get(&self, item_name: &str) -> Option<&T> {
        self.iter().find_map(
            |(name, data)| {
                if item_name == name {
                    Some(data)
                } else {
                    None
                }
            },
        )
    }
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct ResourceItem<T> {
    pub name: String32,
    pub data: T,
}

impl<T> std::fmt::Debug for ResourceChunk<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "NamedResources {{")?;
        for item in self.items.iter().flatten() {
            writeln!(f, "    {:?},", item.name)?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Sound {
    pub ver: u32,
    pub kind: i32,
    pub file_type: String32,
    // if ver == 440 {
    //   if kind != -1 { zlib32 }
    //   _: u64,
    // } else {
    pub file_name: String32,
    pub present: Bool32,
    pub data: Data32, // deflated in ver == 600
    pub effects: u32,
    pub volume: f64,
    pub pan: f64,
    // }
    pub on_demand: Bool32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Sprite {
    pub ver: u32,
    pub width: U32x2,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_bottom: i32,
    pub bbox_top: i32,
    pub transparent: Bool32,
    #[nom(Cond = "ver == 542")]
    pub smooth_edges: Option<Bool32>,
    #[nom(Cond = "ver == 542")]
    pub preload_texture: Option<Bool32>,
    pub bbox: u32,
    pub precise_collision: Bool32,
    #[nom(Cond = "ver == 400")]
    pub use_video_memory: Option<Bool32>,
    #[nom(Cond = "ver == 400")]
    pub on_demand: Option<Bool32>,
    pub origin: U32x2,
    #[nom(LengthCount = "le_u32")]
    pub subimages: Vec<ZlibImage>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Background {
    pub ver: u32,
    pub size: U32x2,
    pub transparent: Bool32,
    #[nom(Cond = "ver == 400")]
    pub use_video_memory: Option<Bool32>,
    #[nom(Cond = "ver == 400")]
    pub on_demand: Option<Bool32>,
    #[nom(Cond = "ver == 543")]
    pub smooth_edges: Option<Bool32>,
    #[nom(Cond = "ver == 543")]
    pub preload_texture: Option<Bool32>,
    #[nom(Cond = "ver >= 543")]
    pub tiling: Option<BackgroundTiling>,
    pub image_exists: Bool32,
    #[nom(Cond = "image_exists == Bool32::True")]
    pub image: Option<ZlibImage>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct BackgroundTiling {
    pub enabled: Bool32,
    pub size: U32x2,
    pub offset: U32x2,
    pub sep: U32x2,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Path {
    pub ver: u32,
    pub kind: u32,
    pub closed: Bool32,
    pub precision: u32,
    pub room_index: i32,
    pub snap: U32x2,
    #[nom(LengthCount = "le_u32")]
    pub points: Vec<Point>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Point {
    pub position: F64x2,
    pub speed: f64,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Script {
    pub ver: u32,
    pub script: String32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Font {
    pub ver: u32,
    pub font_name: String32,
    pub size: u32,
    pub bold: Bool32,
    pub italic: Bool32,
    pub character_range_begin: u32,
    pub character_range_end: u32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Timeline {
    pub ver: u32,
    #[nom(LengthCount = "le_u32")]
    pub moments: Vec<Moment>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Moment {
    pub position: u32,
    pub event_ver: u32,
    #[nom(LengthCount = "le_u32")]
    pub actions: Vec<Action>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Object {
    pub ver: u32,
    pub sprite_index: i32,
    pub solid: Bool32,
    pub visible: Bool32,
    pub depth: u32,
    pub persistent: Bool32,
    pub parent_object_index: i32,
    pub mask_sprite_index: i32,
    pub event_types_count: u32,
    #[nom(Count = "(event_types_count + 1)")]
    pub event_types: Vec<EventType>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct EventType {
    pub events: Vec<Event>,
    #[nom(Verify = "*_end == -1")]
    _end: i32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Event {
    #[nom(Verify = "*number != -1")]
    pub number: i32,
    pub ver: u32,
    #[nom(LengthCount = "le_u32")]
    pub actions: Vec<Action>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Action {
    pub ver: u32,
    pub library_id: u32,
    pub action_id: u32,
    pub kind: ActionKind,
    pub can_be_relative: Bool32,
    pub is_a_question: Bool32,
    pub has_target: Bool32,
    pub exec: ActionExec,
    // #[nom(Cond = "action_exec == ActionExec::Function")]
    pub function_name: String32,
    // #[nom(Cond = "action_exec == ActionExec::Code")]
    pub code: String32,
    pub argument_count: u32,
    #[nom(LengthCount = "le_u32")]
    pub argument_kinds: Vec<ArgumentKind>,
    pub target_object_index: i32,
    pub relative: Bool32,
    #[nom(LengthCount = "le_u32")]
    pub argument_values: Vec<String32>,
    pub not: Bool32,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ActionKind {
    Normal,
    Begin,
    End,
    Else,
    Exit,
    Repeat,
    Variable,
    Code,
    Placeholder,
    Separator,
    Label,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ActionExec {
    None,
    Function,
    Code,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ArgumentKind {
    Expression,
    String,
    Both,
    Boolean,
    Menu,
    Sprite,
    Sound,
    Background,
    Path,
    Script,
    Object,
    Room,
    Font,
    Color,
    Timeline,
    FontString,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Room {
    pub ver: u32,
    pub caption: String32,
    pub size: U32x2,
    pub snap: U32x2,
    pub isometric_grid: Bool32,
    pub speed: u32,
    pub persistent: Bool32,
    pub background_color: u32,
    pub draw_background_color: Bool32,
    pub creation_code: String32,
    #[nom(LengthCount = "le_u32")]
    pub backgrounds: Vec<RoomBackground>,
    pub enable_views: Bool32,
    #[nom(LengthCount = "le_u32")]
    pub views: Vec<RoomView>,
    #[nom(LengthCount = "le_u32")]
    pub instances: Vec<RoomInstance>,
    #[nom(LengthCount = "le_u32")]
    pub tiles: Vec<RoomTile>,
    pub preserve_editor_info: Bool32,
    pub editor_size: U32x2,
    pub editor_show_grid: Bool32,
    pub editor_show_objects: Bool32,
    pub editor_show_tiles: Bool32,
    pub editor_show_backgrounds: Bool32,
    pub editor_show_foregrounds: Bool32,
    pub editor_show_views: Bool32,
    pub editor_delete_underlying_objects: Bool32,
    pub editor_delete_underlying_tiles: Bool32,
    // v520 stuff...
    pub editor_tab: u32,
    pub editor_scroll: U32x2,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct RoomBackground {
    pub visible: Bool32,
    pub foreground_image: Bool32,
    pub background_image_index: i32,
    pub position: U32x2,
    pub tile: U32x2,
    pub speed: U32x2,
    pub stretch: Bool32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct RoomView {
    pub visible: Bool32,
    // ver 541+
    pub view_pos: U32x2,
    pub view_size: U32x2,
    pub port_pos: U32x2,
    pub port_size: U32x2,
    pub border: U32x2,
    pub spacing: I32x2,
    pub following_index: i32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct RoomInstance {
    pub pos: U32x2,
    pub object_index: u32,
    pub id: u32,
    pub creation_code: String32,
    pub locked: Bool32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct RoomTile {
    pub pos: U32x2,
    pub background_index: u32,
    pub tile: U32x2,
    pub size: U32x2,
    pub depth: u32,
    pub id: u32,
    pub locked: Bool32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Chunk<T> {
    pub ver: u32,
    #[nom(LengthCount = "le_u32")]
    pub items: Vec<T>,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Include {
    pub filename: String32,
    pub filepath: String32,
    pub original: Bool32,
    pub original_size: u32,
    _stored: Bool32,
    #[nom(Cond = "_stored == Bool32::True")]
    pub file_data: Option<Data32>,
    pub export_type: u32,
    pub export_folder: String32,
    pub overwrite: Bool32,
    pub free_after_export: Bool32,
    pub remove_at_game_end: Bool32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct GameInformation {
    pub ver: u32,
    pub background_color: Color32,
    // 800+ help_window: Bool32,
    pub reuse_main_style: Bool32,
    pub caption: String32,
    pub position: I32x2,
    pub size: U32x2,
    pub border: Bool32,
    pub resizable: Bool32,
    pub topmost: Bool32,
    pub pause: Bool32,
    pub rtf: String32,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct ResourceTreeItem {
    pub status: ResourceTreeStatus,
    pub kind: ResourceKind,
    pub index: u32,
    pub name: String32,
    #[nom(LengthCount = "le_u32")]
    pub contents: Vec<ResourceTreeItem>,
}

#[repr(u32)]
#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ResourceTreeStatus {
    Primary = 1,
    Group = 2,
    Secondary = 3,
}

#[repr(u32)]
#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ResourceKind {
    None = 0,
    Object = 1,
    Sprite = 2,
    Sound = 3,
    Room = 4,
    Unknown5 = 5,
    Background = 6,
    Script = 7,
    Path = 8,
    Font = 9,
    GameInformation = 10,
    GameSettings = 11,
    Timeline = 12,
    ExtensionPackages = 13,
    Shader = 14,
}

#[derive(NomLE)]
#[nom(GenericErrors)]
pub struct Color32(u32);

impl std::fmt::Debug for Color32 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#{:06x}", self.0)
    }
}

#[derive(NomLE)]
#[nom(GenericErrors)]
pub struct Pair<T>(pub T, pub T);

pub type I32x2 = Pair<i32>;
pub type U32x2 = Pair<u32>;
pub type F64x2 = Pair<f64>;

impl<T: std::fmt::Debug> std::fmt::Debug for Pair<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.0, self.1)
    }
}

fn parse_cond32<'nom, T: Parse<&'nom [u8], E>, E: ParseError<&'nom [u8]>>(
    input: &'nom [u8],
) -> nom::IResult<&'nom [u8], Option<T>, E> {
    flat_map(Bool32::parse, |present| {
        nom::combinator::cond(present == Bool32::True, T::parse)
    })(input)
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Guid(pub [u8; 16]);

impl<'nom, E: ParseError<&'nom [u8]>> Parse<&'nom [u8], E> for Guid {
    fn parse(i: &'nom [u8]) -> nom::IResult<&'nom [u8], Self, E> {
        let (i, d) = take(16usize)(i)?;
        Ok((i, Self(d.try_into().unwrap())))
    }
}

impl std::fmt::Debug for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, NomLE)]
#[nom(GenericErrors)]
pub struct String32(#[nom(Parse = "parse_string")] pub String);

impl std::fmt::Display for String32 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for String32 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn parse_string<'nom, E: ParseError<&'nom [u8]>>(
    input: &'nom [u8],
) -> nom::IResult<&'nom [u8], String, E> {
    nom::combinator::map(flat_map(le_u32, take), |x: &[u8]| {
        String::from_utf8(x.to_vec()).unwrap()
    })(input)
}

fn parse_inflate<'nom, E: ParseError<&'nom [u8]>>(
    input: &'nom [u8],
) -> nom::IResult<&'nom [u8], Vec<u8>, E> {
    let (input, data) = flat_map(le_u32, take)(input)?;
    let data = inflate::inflate_bytes_zlib(data).map_err(|error| {
        eprintln!("inflate failed: {error}");
        nom::Err::Failure(E::from_error_kind(data, nom::error::ErrorKind::Verify))
    })?;
    Ok((input, data))
}

#[derive(NomLE)]
#[nom(GenericErrors)]
pub struct Data32 {
    length: u32,
    #[nom(Map = "Into::into", Take = "length")]
    pub data: Vec<u8>,
}

impl std::fmt::Debug for Data32 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("length", &self.length)
            .finish_non_exhaustive()
    }
}

#[derive(NomLE)]
#[nom(GenericErrors)]
pub struct ZlibImage {
    _present: i32,
    #[nom(Cond = "_present != -1", Parse = "parse_inflate")]
    pub data: Option<Vec<u8>>,
}

impl std::fmt::Debug for ZlibImage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ZlibImage")
            .field("length", &self.data.as_ref().map(|d| d.len()))
            .finish_non_exhaustive()
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum Bool32 {
    False,
    True,
}
