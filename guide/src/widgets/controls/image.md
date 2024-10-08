# Image

The [`Image`][image] widget displays an image/texture with configurable scaling
options.

## ImageCornerRadius

Use [`ImageCornerRadius`][ImageCornerRadius] component to set the corner/border radius of an image. `CornerRadius` is not used.

```rs
image
  .with_dynamic(&ImageCornerRadius, CornerRadius) // CornerRadius is a Dynamic(Lp) here
  .with(&ImageCornerRadius, Lp::points(6)) // or, a static corner radius
```

## Dynamic textures

Use a [`Dynamic`][Dynamic]`(`[`AnyTexture`][AnyTexture]`)`.

You can default to a empty texture like so:

```rs
let dynamic_texture = Dynamic::new(
    AnyTexture::Lazy(
        LazyTexture::from_image(
            image::DynamicImage::ImageRgba8(
                image::ImageBuffer::new(1, 1)
            ),
            cushy::kludgine::wgpu::FilterMode::Linear
        )
    )
);
let widget = Image::new(dynamic_texture); // Creates image widget with an empty texture, that can later be changed
```

To load an image from bytes, use the [`image`][image-crate] crate and then pass it to LazyTexture:

```rs
let image = image::load_from_memory(&bytes).unwrap();
let texture = LazyTexture::from_image(
    image,
    cushy::kludgine::wgpu::FilterMode::Linear
);
let texture = AnyTexture::Lazy(texture);
dynamic_texture.set(texture);
```

## FilterMode

[`FilterMode`][FilterMode] enum specifies the sampler to be used for when an image is scaled.  
Linear smooths the version, making it blurry at the extremes but with less grain.  
Nearest makes the result more pixelated with hard edges.

[image]: <{{ docs }}/widgets/image/struct.Image.html>
[image-crate]: <https://docs.rs/image/latest/image/>
[ImageCornerRadius]: <{{ docs }}/widgets/image/struct.ImageCornerRadius.html>
[FilterMode]: <https://docs.rs/wgpu-types/22.0.0/wgpu_types/enum.FilterMode.html>
[Dynamic]: </about/reactive.html#what-is-a-dynamict>
[AnyTexture]: <https://docs.rs/kludgine/latest/kludgine/enum.AnyTexture.html>
