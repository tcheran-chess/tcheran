cfg_select! {
    feature = "syzygy" => {
        #[allow(
            unused,
            nonstandard_style,
            clippy::all,
            clippy::use_self,
            reason = "Generated code"
        )]
        mod bindings;

        mod tablebases;
        pub use tablebases::*;
    }
    _ => {
        use crate::{chess::prelude::*, engine::search::RootTbInfo};

        #[derive(Clone)]
        pub struct Tablebase;

        impl Tablebase {
            pub fn new() -> Self {
                Self
            }

            pub fn set_paths(&mut self, _path: &str) {}

            pub fn can_probe(&self, _game: &Game) -> bool {
                false
            }

            pub fn n_men(&self) -> u8 {
                0
            }

            pub fn wdl(&self, _game: &Game) -> Option<Outcome> {
                None
            }

            pub fn root_wdl_dtz(&self, _game: &Game) -> Option<RootTbInfo> {
                None
            }
        }
    }
}
