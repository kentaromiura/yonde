//use std::path::Path;

use anyhow::Result;

pub struct WeightedTokens {
    tensors: safetensors::SafeTensors<'static>,
}

impl WeightedTokens {
    pub fn load_safetensors_from_bytes(data: &'static [u8]) -> Result<Self> {
        let tensors = unsafe { safetensors::SafeTensors::deserialize(std::mem::transmute(data))? };
        Ok(Self { tensors })
    }

    // pub fn load_safetensors(path: &Path) -> Result<Self> {
    //     let data = std::fs::read(path)?;
    //     let tensors =
    //         unsafe { safetensors::SafeTensors::deserialize(std::mem::transmute(data.as_slice()))? };
    //     Ok(Self { tensors })
    // }

    pub fn get_float_tensor(&self, name: &str) -> Result<Vec<f32>> {
        let tensor = self.tensors.tensor(name)?;

        let dtype = tensor.dtype();
        let data = tensor.data();

        match dtype {
            safetensors::Dtype::F32 => {
                let floats: Vec<f32> = data
                    .chunks_exact(4)
                    .map(|chunk| {
                        let mut arr = [0u8; 4];
                        arr.copy_from_slice(chunk);
                        f32::from_le_bytes(arr)
                    })
                    .collect();
                Ok(floats)
            }
            safetensors::Dtype::F16 => {
                let floats: Vec<f32> = data
                    .chunks_exact(2)
                    .map(|chunk| {
                        let mut arr = [0u8; 2];
                        arr.copy_from_slice(chunk);
                        half::f16::from_le_bytes(arr).to_f32()
                    })
                    .collect();
                Ok(floats)
            }
            _ => anyhow::bail!("Expected f32 or f16 tensor, got {:?}", dtype),
        }
    }

    pub fn list_tensors(&self) -> Vec<String> {
        self.tensors
            .tensors()
            .iter()
            .map(|(k, _)| k.clone())
            .collect()
    }
}
