//mod hf_hub;
mod weights;

pub mod comic_text_detector;
pub mod manga_ocr;

//pub use hf_hub::set_cache_dir;

pub type B = burn::backend::Wgpu<f32>;

pub type Dev = <B as burn::tensor::backend::Backend>::Device;

pub fn device() -> Dev {
    <B as burn::tensor::backend::Backend>::Device::default()
}

pub fn cuda_is_available() -> bool {
    (unsafe {
        libloading::Library::new(if cfg!(target_os = "windows") {
            "nvcuda.dll"
        } else {
            "libcuda.so"
        })
        .is_ok()
    }) && cfg!(feature = "cuda")
}

pub fn metal_is_available() -> bool {
    cfg!(feature = "metal")
}
