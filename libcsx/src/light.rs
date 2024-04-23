use dif::types::ColorI;

use crate::csx;

#[derive(Copy, Clone)]
pub enum Light {
    Point {
        color: ColorI,
        intensity: f32,
        falloff_inner: f32,
        falloff_outer: f32,
    },
    SpotLight {
        color: ColorI,
        intensity: f32,
        falloff_inner: f32,
        falloff_outer: f32,
        heading: f32,
        pitch: f32,
        angle_inner: f32,
        angle_outer: f32,
    },
    EmitterPoint {
        falloff_type: u32,
        falloff1: f32,
        falloff2: f32,
        falloff3: f32,
    },
    EmitterSpot {
        falloff_type: u32,
        falloff1: f32,
        falloff2: f32,
        falloff3: f32,
        theta: f32,
        phi: f32,
    },
    Flicker {
        color: [ColorI; 5],
        speed: f32,
        falloff1: f32,
        falloff2: f32,
        spawnflags: u32,
    },
    Omni {
        color: ColorI,
        falloff1: f32,
        falloff2: f32,
    },
    Pulse {
        color: [ColorI; 2],
        speed: f32,
        falloff1: f32,
        falloff2: f32,
        spawnflags: u32,
    },
    Pulse2 {
        color: [ColorI; 2],
        falloff1: f32,
        falloff2: f32,
        attack: f32,
        decay: f32,
        sustain1: f32,
        sustain2: f32,
        spawnflags: u32,
    },
    Runway {
        color: ColorI,
        speed: f32,
        pingpong: bool,
        spawnflags: u32,
        steps: u32,
        falloff1: f32,
        falloff2: f32,
    },
    Spot {
        color: ColorI,
        falloff1: f32,
        falloff2: f32,
        distance1: f32,
        distance2: f32,
    },
    Strobe {
        color: [ColorI; 2],
        speed: f32,
        spawnflags: u32,
        falloff1: f32,
        falloff2: f32,
    },
}

fn make_color(v: Vec<u8>) -> ColorI {
    ColorI {
        r: v[0],
        g: v[1],
        b: v[2],
        a: 255,
    }
}

impl Light {
    pub fn new(ent: &csx::Entity) -> Self {
        match ent.classname.as_str() {
            "light_point" => Light::Point {
                color: make_color(
                    ent.properties
                        .get("color")
                        .unwrap_or(&"255 255 255".to_string())
                        .trim()
                        .split(' ')
                        .map(|v| v.parse::<u8>().unwrap())
                        .collect::<Vec<u8>>(),
                ),
                intensity: ent
                    .properties
                    .get("intensity")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                falloff_inner: ent
                    .properties
                    .get("falloff_inner")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
                falloff_outer: ent
                    .properties
                    .get("falloff_outer")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
            },
            "light_spotlight" => Light::SpotLight {
                color: make_color(
                    ent.properties
                        .get("color")
                        .unwrap_or(&"255 255 255".to_string())
                        .trim()
                        .split(' ')
                        .map(|v| v.parse::<u8>().unwrap())
                        .collect::<Vec<u8>>(),
                ),
                intensity: ent
                    .properties
                    .get("intensity")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                falloff_inner: ent
                    .properties
                    .get("falloff_inner")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
                falloff_outer: ent
                    .properties
                    .get("falloff_outer")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                heading: ent
                    .properties
                    .get("heading")
                    .unwrap_or(&"0.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.0),
                pitch: ent
                    .properties
                    .get("pitch")
                    .unwrap_or(&"0.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.0),
                angle_inner: ent
                    .properties
                    .get("angle_inner")
                    .unwrap_or(&"30.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(30.0),
                angle_outer: ent
                    .properties
                    .get("angle_outer")
                    .unwrap_or(&"60.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(60.0),
            },
            "light_emitter_point" => Light::EmitterPoint {
                falloff_type: ent
                    .properties
                    .get("falloff_type")
                    .unwrap_or(&"0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"0.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff3: ent
                    .properties
                    .get("falloff3")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
            },
            "light_emitter_spot" => Light::EmitterSpot {
                falloff_type: ent
                    .properties
                    .get("falloff_type")
                    .unwrap_or(&"0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"0.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff3: ent
                    .properties
                    .get("falloff3")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                theta: ent
                    .properties
                    .get("theta")
                    .unwrap_or(&"0.2".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.2),
                phi: ent
                    .properties
                    .get("phi")
                    .unwrap_or(&"0.4".to_string())
                    .parse::<f32>()
                    .unwrap_or(0.4),
            },
            "light_flicker" => Light::Flicker {
                color: [
                    make_color(
                        ent.properties
                            .get("color1")
                            .unwrap_or(&"255 255 255".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color2")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color3")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color4")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color5")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                ],
                speed: ent
                    .properties
                    .get("speed")
                    .unwrap_or(&"2.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(2.0),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                spawnflags: ent
                    .properties
                    .get("spawnflags")
                    .unwrap_or(&"3".to_string())
                    .parse::<u32>()
                    .unwrap_or(3),
            },
            "light_omni" => Light::Omni {
                color: make_color(
                    ent.properties
                        .get("color")
                        .unwrap_or(&"255 255 255".to_string())
                        .trim()
                        .split(' ')
                        .map(|v| v.parse::<u8>().unwrap())
                        .collect::<Vec<u8>>(),
                ),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"1000.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1000.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"200.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(200.0),
            },
            "light_pulse" => Light::Pulse {
                color: [
                    make_color(
                        ent.properties
                            .get("color1")
                            .unwrap_or(&"255 255 255".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color2")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                ],
                speed: ent
                    .properties
                    .get("speed")
                    .unwrap_or(&"2.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(2.0),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                spawnflags: ent
                    .properties
                    .get("spawnflags")
                    .unwrap_or(&"3".to_string())
                    .parse::<u32>()
                    .unwrap_or(3),
            },
            "light_pulse2" => Light::Pulse2 {
                color: [
                    make_color(
                        ent.properties
                            .get("color1")
                            .unwrap_or(&"255 255 255".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color2")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                ],
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                spawnflags: ent
                    .properties
                    .get("spawnflags")
                    .unwrap_or(&"3".to_string())
                    .parse::<u32>()
                    .unwrap_or(3),
                attack: ent
                    .properties
                    .get("attack")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
                decay: ent
                    .properties
                    .get("decay")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
                sustain1: ent
                    .properties
                    .get("sustain1")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
                sustain2: ent
                    .properties
                    .get("sustain2")
                    .unwrap_or(&"1.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(1.0),
            },
            "light_runway" => Light::Runway {
                color: make_color(
                    ent.properties
                        .get("color")
                        .unwrap_or(&"255 255 255".to_string())
                        .trim()
                        .split(' ')
                        .map(|v| v.parse::<u8>().unwrap())
                        .collect::<Vec<u8>>(),
                ),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                speed: ent
                    .properties
                    .get("speed")
                    .unwrap_or(&"2.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(2.0),
                pingpong: ent
                    .properties
                    .get("pingpong")
                    .unwrap_or(&"0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0)
                    == 1,
                spawnflags: ent
                    .properties
                    .get("spawnflags")
                    .unwrap_or(&"3".to_string())
                    .parse::<u32>()
                    .unwrap_or(3),
                steps: ent
                    .properties
                    .get("steps")
                    .unwrap_or(&"0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0),
            },
            "light_spot" => Light::Spot {
                color: make_color(
                    ent.properties
                        .get("color")
                        .unwrap_or(&"255 255 255".to_string())
                        .trim()
                        .split(' ')
                        .map(|v| v.parse::<u8>().unwrap())
                        .collect::<Vec<u8>>(),
                ),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                distance1: ent
                    .properties
                    .get("distance1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                distance2: ent
                    .properties
                    .get("distance2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
            },
            "light_strobe" => Light::Strobe {
                color: [
                    make_color(
                        ent.properties
                            .get("color1")
                            .unwrap_or(&"255 255 255".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                    make_color(
                        ent.properties
                            .get("color2")
                            .unwrap_or(&"0 0 0".to_string())
                            .trim()
                            .split(' ')
                            .map(|v| v.parse::<u8>().unwrap())
                            .collect::<Vec<u8>>(),
                    ),
                ],
                speed: ent
                    .properties
                    .get("speed")
                    .unwrap_or(&"2.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(2.0),
                falloff1: ent
                    .properties
                    .get("falloff1")
                    .unwrap_or(&"10.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(10.0),
                falloff2: ent
                    .properties
                    .get("falloff2")
                    .unwrap_or(&"100.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(100.0),
                spawnflags: ent
                    .properties
                    .get("spawnflags")
                    .unwrap_or(&"3".to_string())
                    .parse::<u32>()
                    .unwrap_or(3),
            },

            _ => panic!("Invalid light type: {}", ent.classname),
        }
    }
}
