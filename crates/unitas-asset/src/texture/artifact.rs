use crate::texture::import::{
    import_texture, import_texture_for_target, TextureFormat, TextureImportError,
    TextureImportResult, TextureImportSettings,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TextureArtifact {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub pixels: Vec<u8>,
    pub srgb: bool,
    pub normal_map: bool,
    pub generate_mips: bool,
    pub settings: TextureImportSettings,
}

impl TextureArtifact {
    pub fn from_import(result: &TextureImportResult) -> Self {
        TextureArtifact {
            width: result.texture.width,
            height: result.texture.height,
            format: result.texture.format,
            pixels: result.texture.pixels.clone(),
            srgb: result.settings.srgb,
            normal_map: result.settings.is_normal_map(),
            generate_mips: result.settings.mipmaps,
            settings: result.settings.clone(),
        }
    }
}

pub fn texture_artifact(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
) -> Result<TextureArtifact, TextureImportError> {
    let result = import_texture(bytes, file_name, settings)?;
    Ok(TextureArtifact::from_import(&result))
}

pub fn texture_artifact_for_target(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
    build_target: Option<&str>,
) -> Result<TextureArtifact, TextureImportError> {
    let result = import_texture_for_target(bytes, file_name, settings, build_target)?;
    Ok(TextureArtifact::from_import(&result))
}
