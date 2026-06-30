pub mod eval;
pub mod options;
pub mod uci;
pub mod util;

pub mod see;

mod cuckoo;
pub mod params;
pub mod search;
pub mod tablebases;
pub mod transposition_table;
pub mod tuning;

pub fn init() {
    cuckoo::init();
    search::init();
}
