use csx::builder::ProgressEventListener;
use csx::convert_csx_to_dif;
use csx::set_convert_configuration;
use dif::io::EngineVersion;
use js_sys::Array;
use serde::Serialize;
use wasm_bindgen::prelude::*;
pub use wasm_bindgen_rayon::init_thread_pool;

struct JSListener {
    pub js_callback: js_sys::Function,
}

impl ProgressEventListener for JSListener {
    fn progress(&mut self, current: u32, total: u32, status: String, finish_status: String) {
        let args_vec = vec![
            JsValue::from(current),
            JsValue::from(total),
            JsValue::from(status),
            JsValue::from(finish_status),
        ];
        self.js_callback
            .apply(&JsValue::NULL, &Array::from_iter(args_vec.iter()))
            .unwrap();
    }
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct BSPReport {
    pub hit: i32,
    pub total: usize,
    pub surface_area_percentage: f32,
    pub balance_factor: i32,
}

#[derive(Serialize)]
pub struct CSXConvertOutput {
    pub data: Vec<serde_bytes::ByteBuf>,
    pub bsp_reports: Vec<BSPReport>,
}

#[wasm_bindgen]
pub fn convert_csx(
    csxbuf: &str,
    engine_ver_str: &str,
    interior_version: u32,
    mb: bool,
    bsp_type: u32,
    epsilon_point: f32,
    epsilon_plane: f32,
    js_callback: js_sys::Function,
) -> JsValue {
    let engine_ver = match engine_ver_str {
        "MBG" => EngineVersion::MBG,
        "TGE" => EngineVersion::TGE,
        "TGEA" => EngineVersion::TGEA,
        "T3D" => EngineVersion::T3D,
        _ => EngineVersion::Unknown,
    };

    unsafe {
        set_convert_configuration(
            mb,
            epsilon_point,
            epsilon_plane,
            match bsp_type {
                0 => csx::bsp::SplitMethod::Exhaustive,
                1 => csx::bsp::SplitMethod::Fast,
                2.. => csx::bsp::SplitMethod::None,
            },
        )
    };

    let mut silent_listener = JSListener { js_callback };
    let (results, reports) = convert_csx_to_dif(
        csxbuf.to_owned(),
        engine_ver,
        interior_version,
        &mut silent_listener,
    );
    let reports_wasm = reports
        .iter()
        .map(|r| BSPReport {
            hit: r.hit,
            total: r.total,
            surface_area_percentage: r.hit_area_percentage,
            balance_factor: r.balance_factor,
        })
        .collect::<Vec<_>>();

    let results_bb = results
        .into_iter()
        .map(|r| serde_bytes::ByteBuf::from(r))
        .collect::<Vec<_>>();

    let output_val = CSXConvertOutput {
        data: results_bb,
        bsp_reports: reports_wasm,
    };

    serde_wasm_bindgen::to_value(&output_val).unwrap()
}
