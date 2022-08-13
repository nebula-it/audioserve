use anyhow::Result;
use headers::{ContentLength, ContentType};
use hyper::{Body, Response};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use std::{
    io::{Cursor, Read},
    path::Path,
};

use crate::{config::get_config, util::ResponseBuilderExt};

use self::cache::{cache_icon, cached_icon};

pub mod cache;

pub fn icon_response(path: impl AsRef<Path>) -> Result<Response<Body>> {
    let cache_enabled = !get_config().icons.cache_disabled;
    let data = match if cache_enabled {
        cached_icon(&path)
    } else {
        None
    } {
        Some(mut f) => {
            let mut data = Vec::with_capacity(1024);
            f.read_to_end(&mut data)?;
            data
        }
        None => {
            let data = scale_cover(&path)?;
            if cache_enabled {
                cache_icon(path, &data)
                    .unwrap_or_else(|e| error!("error adding icon to cache: {}", e));
            }
            data
        }
    };

    Response::builder()
        .status(200)
        .typed_header(ContentLength(data.len() as u64))
        .typed_header(ContentType::png())
        .body(data.into())
        .map_err(anyhow::Error::from)
}

pub fn scale_cover(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    use image::imageops::FilterType;
    let img = ImageReader::open(path)?.decode()?;
    let sz = get_config().icons.size;
    let scaled = img.resize(
        sz,
        sz,
        if !get_config().icons.fast_scaling {
            FilterType::Lanczos3
        } else {
            FilterType::Triangle
        },
    );
    let mut data = Vec::with_capacity(1024);
    let mut buf = Cursor::new(&mut data);
    scaled.write_to(&mut buf, ImageOutputFormat::Png)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init::init_default_config;
    #[test]
    fn test_scale_image() -> anyhow::Result<()> {
        init_default_config();
        let mut data = scale_cover("test_data/cover.jpg")?;
        let mut buf = Cursor::new(&mut data);
        let img2 = ImageReader::with_format(&mut buf, image::ImageFormat::Png).decode()?;
        let sz = get_config().icons.size;
        assert_eq!(sz, img2.width());
        assert_eq!(sz, img2.height());
        assert!(data.len() > 1024);
        Ok(())
    }
}
