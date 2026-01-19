pub mod app_animation;
pub mod main_process;
pub mod message_results;
pub mod print_benchmarking;
pub mod sub_process_creation;
pub mod update_channel_live_status;
pub mod update_channel_livestreams;

pub use main_process::run_main_process;
pub use message_results::process_irc_message_results;
pub use print_benchmarking::print_benchmarking;
pub use sub_process_creation::create_sub_processes;
pub use update_channel_live_status::update_channel_live_statuses;
