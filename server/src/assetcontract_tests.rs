use crate::assetcontract::{Asset, Profile, style_check, validate};
use flate2::{Compression, write::ZlibEncoder};
use serde_json::json;
use std::io::Write;

fn crc(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for b in data {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend(data);
    out.extend(&body);
    out.extend(crc(&body).to_be_bytes());
}

/// An 8-bit RGBA PNG whose pixel (x, y) is f(x, y).
fn png(w: u32, h: u32, color_type: u8, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let bpp = if color_type == 6 { 4 } else { 3 };
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.extend(&f(x, y)[..bpp]);
        }
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(&raw).unwrap();
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, color_type, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z.finish().unwrap());
    chunk(&mut out, b"IEND", &[]);
    out
}

fn solid(w: u32, h: u32, c: [u8; 4]) -> Vec<u8> {
    png(w, h, 6, |_, _| c)
}

fn wav(rate: u32, channels: u16, bits: u16, format: u16) -> Vec<u8> {
    let mut o = b"RIFF".to_vec();
    o.extend(36u32.to_le_bytes());
    o.extend(b"WAVEfmt ");
    o.extend(16u32.to_le_bytes());
    o.extend(format.to_le_bytes());
    o.extend(channels.to_le_bytes());
    o.extend(rate.to_le_bytes());
    o.extend((rate * channels as u32 * bits as u32 / 8).to_le_bytes());
    o.extend((channels * bits / 8).to_le_bytes());
    o.extend(bits.to_le_bytes());
    o.extend(b"data");
    o.extend(0u32.to_le_bytes());
    o
}

/// An Ogg page holding a Vorbis identification header.
fn ogg(rate: u32, channels: u8) -> Vec<u8> {
    let mut id = vec![1];
    id.extend(b"vorbis");
    id.extend(0u32.to_le_bytes());
    id.push(channels);
    id.extend(rate.to_le_bytes());
    id.extend([0u8; 12]);
    let mut o = b"OggS".to_vec();
    o.extend([0, 2]);
    o.extend([0u8; 8 + 4 + 4 + 4]);
    o.push(1);
    o.push(id.len() as u8);
    o.extend(id);
    o
}

/// A GLB with one mesh per entry of `tris`, each with an index accessor of 3 * tris indices.
fn glb(tris: &[u32]) -> Vec<u8> {
    let accessors: Vec<_> = tris.iter().map(|t| json!({"count": t * 3, "componentType": 5123, "type": "SCALAR"})).collect();
    let meshes: Vec<_> = (0..tris.len()).map(|i| json!({"primitives": [{"attributes": {"POSITION": 0}, "indices": i}]})).collect();
    let mut text = json!({"asset": {"version": "2.0"}, "accessors": accessors, "meshes": meshes}).to_string().into_bytes();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let mut o = b"glTF".to_vec();
    o.extend(2u32.to_le_bytes());
    o.extend((12 + 8 + text.len() as u32).to_le_bytes());
    o.extend((text.len() as u32).to_le_bytes());
    o.extend(b"JSON");
    o.extend(text);
    o
}

fn a(path: &str, bytes: Vec<u8>) -> Asset {
    Asset { path: path.into(), bytes }
}

fn rules(assets: &[Asset]) -> Vec<String> {
    let p = Profile::named("kk-engine").unwrap();
    validate(&p, assets).into_iter().map(|i| format!("{}:{}", i.path, i.rule)).collect()
}

#[test]
fn profile_lookup() {
    assert!(Profile::named("kk-engine").is_some());
    assert!(Profile::named("unity").is_none());
}

#[test]
fn clean_assets_pass() {
    let good = vec![
        a("sprites/hero_idle.png", solid(32, 32, [200, 10, 10, 255])),
        a("audio/jump.wav", wav(44100, 1, 16, 1)),
        a("audio/theme.ogg", ogg(48000, 2)),
        a("models/rock.glb", glb(&[500])),
    ];
    assert_eq!(rules(&good), Vec::<String>::new());
}

#[test]
fn bad_names_are_refused_with_a_reason() {
    let p = Profile::named("kk-engine").unwrap();
    let issues = validate(&p, &[a("sprites/Hero Idle.png", solid(8, 8, [1, 2, 3, 255])), a("sprites/ok_name.png", solid(8, 8, [1, 2, 3, 255]))]);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].rule, "name");
    assert!(issues[0].message.contains("lowercase"), "{}", issues[0].message);
    assert_eq!(rules(&[a("../escape.png", solid(8, 8, [0; 4]))]), vec!["../escape.png:name"]);
}

#[test]
fn unsupported_types_are_refused() {
    assert_eq!(rules(&[a("art/hero.psd", vec![1, 2, 3])]), vec!["art/hero.psd:type"]);
    assert_eq!(rules(&[a("audio/song.mp3", vec![1, 2, 3])]), vec!["audio/song.mp3:type"]);
}

#[test]
fn png_must_be_a_real_8_bit_png_within_the_size_limit() {
    assert_eq!(rules(&[a("s/broken.png", vec![1, 2, 3, 4])]), vec!["s/broken.png:png"]);
    assert_eq!(rules(&[a("s/huge.png", solid(4097, 1, [0, 0, 0, 255]))]), vec!["s/huge.png:png"]);
    let mut sixteen = solid(4, 4, [0, 0, 0, 255]);
    sixteen[24] = 16; // bit depth in IHDR
    assert_eq!(rules(&[a("s/deep.png", sixteen)]), vec!["s/deep.png:png"]);
    assert_eq!(rules(&[a("s/rgb.png", png(4, 4, 2, |_, _| [9, 9, 9, 0]))]), Vec::<String>::new());
}

#[test]
fn an_atlas_needs_its_frame_metadata_inside_the_image() {
    let sheet = solid(64, 32, [5, 5, 5, 255]);
    assert_eq!(rules(&[a("s/hero_atlas.png", sheet.clone())]), vec!["s/hero_atlas.png:atlas"]);
    let frames = json!({"frames": [{"name": "idle_0", "x": 0, "y": 0, "w": 32, "h": 32}, {"name": "idle_1", "x": 32, "y": 0, "w": 32, "h": 32}]});
    let ok = vec![a("s/hero_atlas.png", sheet.clone()), a("s/hero_atlas.json", frames.to_string().into_bytes())];
    assert_eq!(rules(&ok), Vec::<String>::new());
    let outside = json!({"frames": [{"name": "idle_0", "x": 40, "y": 0, "w": 32, "h": 32}]});
    assert_eq!(rules(&[a("s/hero_atlas.png", sheet.clone()), a("s/hero_atlas.json", outside.to_string().into_bytes())]), vec!["s/hero_atlas.json:atlas"]);
    let dup = json!({"frames": [{"name": "x", "x": 0, "y": 0, "w": 8, "h": 8}, {"name": "x", "x": 8, "y": 0, "w": 8, "h": 8}]});
    assert_eq!(rules(&[a("s/hero_atlas.png", sheet), a("s/hero_atlas.json", dup.to_string().into_bytes())]), vec!["s/hero_atlas.json:atlas"]);
    assert_eq!(rules(&[a("s/not_json.json", b"{oops".to_vec())]), vec!["s/not_json.json:atlas"]);
}

#[test]
fn audio_formats_and_rates() {
    assert_eq!(rules(&[a("a/low.wav", wav(8000, 1, 16, 1))]), vec!["a/low.wav:audio"]);
    assert_eq!(rules(&[a("a/float.wav", wav(44100, 1, 32, 3))]), vec!["a/float.wav:audio"]);
    assert_eq!(rules(&[a("a/surround.wav", wav(44100, 6, 16, 1))]), vec!["a/surround.wav:audio"]);
    assert_eq!(rules(&[a("a/odd.ogg", ogg(22050, 2))]), vec!["a/odd.ogg:audio"]);
    assert_eq!(rules(&[a("a/fake.ogg", b"not ogg at all".to_vec())]), vec!["a/fake.ogg:audio"]);
}

#[test]
fn glb_header_triangle_budget_and_lods() {
    assert_eq!(rules(&[a("m/bad.glb", vec![0; 40])]), vec!["m/bad.glb:glb"]);
    assert_eq!(rules(&[a("m/heavy.glb", glb(&[150_000]))]), vec!["m/heavy.glb:glb"]);
    let lods = vec![a("m/tree_lod0.glb", glb(&[1000])), a("m/tree_lod1.glb", glb(&[400])), a("m/tree_lod2.glb", glb(&[100]))];
    assert_eq!(rules(&lods), Vec::<String>::new());
    let not_smaller = vec![a("m/tree_lod0.glb", glb(&[1000])), a("m/tree_lod1.glb", glb(&[1000]))];
    assert_eq!(rules(&not_smaller), vec!["m/tree_lod1.glb:lod"]);
    let gap = vec![a("m/bush_lod0.glb", glb(&[1000])), a("m/bush_lod2.glb", glb(&[100]))];
    assert_eq!(rules(&gap), vec!["m/bush_lod2.glb:lod"]);
    let no_zero = vec![a("m/fern_lod1.glb", glb(&[100]))];
    assert_eq!(rules(&no_zero), vec!["m/fern_lod1.glb:lod"]);
}

#[test]
fn one_off_model_sprite_is_flagged_in_a_set() {
    // Dark muted pixel-art knights, plus one bright noisy painting.
    let knight = |tone: u8| png(32, 32, 6, move |x, y| if (x + y) % 5 == 0 { [tone, tone / 2, 30, 255] } else { [tone / 2, 20, 20, 255] });
    let mut set: Vec<Asset> = (0..4).map(|i| a(&format!("s/knight_{i}.png"), knight(80 + i as u8 * 6))).collect();
    set.push(a("s/painted.png", png(32, 32, 6, |x, y| [(x * 8) as u8, 230, (y * 8) as u8, 255])));
    let issues = style_check(&set);
    assert_eq!(issues.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["s/painted.png"]);
    assert_eq!(issues[0].rule, "style");
    assert!(!issues[0].message.is_empty());
}

#[test]
fn consistent_sets_and_small_sets_are_not_flagged() {
    let tone = |t: u8| solid(16, 16, [t, t, t, 255]);
    let set: Vec<Asset> = (0..4).map(|i| a(&format!("s/gray_{i}.png"), tone(100 + i * 5))).collect();
    assert!(style_check(&set).is_empty());
    let two = vec![a("s/a.png", tone(10)), a("s/b.png", tone(240))];
    assert!(style_check(&two).is_empty());
    // Different folders are different sets.
    let split = vec![a("x/a.png", tone(10)), a("x/b.png", tone(12)), a("x/c.png", tone(11)), a("y/d.png", tone(250))];
    assert!(style_check(&split).is_empty());
}
