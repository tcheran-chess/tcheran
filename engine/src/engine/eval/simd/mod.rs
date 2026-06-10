cfg_select! {
    target_feature = "avx512bw" => {
        mod avx512;
        pub use avx512::*;
    },
    target_feature = "avx2" => {
        mod avx2;
        pub use avx2::*;
    }
    target_feature = "neon" => {
        mod neon;
        pub use neon::*;
    }
    _ => {}
}
