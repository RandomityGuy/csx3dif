use cgmath::MetricSpace;
use dif::types::{ColorI, Point3F};

use crate::csx;

#[derive(Copy, Clone)]
pub enum Light {
    Point {
        position: Point3F,
        color: ColorI,
        intensity: f32,
        falloff_inner: f32,
        falloff_outer: f32,
    },
    SpotLight {
        position: Point3F,
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
        position: Point3F,
        falloff_type: u32,
        falloff1: f32,
        falloff2: f32,
        falloff3: f32,
    },
    EmitterSpot {
        position: Point3F,
        falloff_type: u32,
        falloff1: f32,
        falloff2: f32,
        falloff3: f32,
        theta: f32,
        phi: f32,
    },
    Flicker {
        position: Point3F,
        color: [ColorI; 5],
        speed: f32,
        falloff1: f32,
        falloff2: f32,
        spawnflags: u32,
    },
    Omni {
        position: Point3F,
        color: ColorI,
        falloff1: f32,
        falloff2: f32,
    },
    Pulse {
        position: Point3F,
        color: [ColorI; 2],
        speed: f32,
        falloff1: f32,
        falloff2: f32,
        spawnflags: u32,
    },
    Pulse2 {
        position: Point3F,
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
        position: Point3F,
        color: ColorI,
        speed: f32,
        pingpong: bool,
        spawnflags: u32,
        steps: u32,
        falloff1: f32,
        falloff2: f32,
    },
    Spot {
        position: Point3F,
        color: ColorI,
        falloff1: f32,
        falloff2: f32,
        distance1: f32,
        distance2: f32,
    },
    Strobe {
        position: Point3F,
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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
                position: ent
                    .origin
                    .unwrap_or(Point3F {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    })
                    .clone(),
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

    pub fn calculate_intensity(&self, pt: &Point3F) -> f32 {
        match self {
            Light::Point {
                position,
                color,
                intensity,
                falloff_inner,
                falloff_outer,
            } => {
                let len = position.distance(*pt);
                if len > *falloff_outer || len < *falloff_inner {
                    return 0.0;
                }
                let intensity = if (len > *falloff_inner) {
                    1.0 - ((len - *falloff_inner) / (*falloff_outer - *falloff_inner))
                } else {
                    1.0
                };

                intensity
            }
            Light::Omni {
                position,
                color,
                falloff1,
                falloff2,
            } => {
                let len = position.distance(*pt);
                if len > *falloff2 || len < *falloff1 {
                    return 0.0;
                }
                let intensity = if (len > *falloff1) {
                    1.0 - ((len - *falloff1) / (*falloff2 - *falloff1))
                } else {
                    1.0
                };

                intensity
            }
            _ => panic!("Not implemented!"),
        }
    }

    pub fn get_base_color(&self) -> Point3F {
        match self {
            Light::Point { color, .. } => Point3F {
                x: color.r as f32 / 255.0,
                y: color.g as f32 / 255.0,
                z: color.b as f32 / 255.0,
            },
            Light::Omni { color, .. } => Point3F {
                x: color.r as f32 / 255.0,
                y: color.g as f32 / 255.0,
                z: color.b as f32 / 255.0,
            },
            _ => panic!("Not implemented!"),
        }
    }

    pub fn get_position(&self) -> Point3F {
        match self {
            Light::Point { position, .. } => *position,
            Light::Omni { position, .. } => *position,
            _ => panic!("Not implemented!"),
        }
    }
}
