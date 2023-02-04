use mini_tiktok_user::{block_on, start_up};

fn main() {
    block_on(start_up()).unwrap().unwrap()
}
