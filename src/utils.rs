#[rustfmt::skip]
macro_rules! wrap {
    ($wrapper:ty, $inner:ty) => {
        impl ::core::ops::Deref for $wrapper {
            type Target = $inner;

            fn deref(&self) -> &Self::Target { &self.inner }
        }
    }
}

macro_rules! include_spv {
    ($device:expr, $($tokens:tt)*) => {{
        let blob = include_bytes!($($tokens)*);
        let code = ::ash::util::read_spv(&mut ::std::io::Cursor::new(blob)).unwrap();
        let info = ::ash::vk::ShaderModuleCreateInfo::default().code(&code);

        $device.create_shader_module(&info, None).unwrap()
    }}
}

pub(crate) use {include_spv, wrap};
