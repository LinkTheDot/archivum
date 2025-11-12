pub use sea_orm_migration::prelude::*;

mod m20250210_025922_twitch_user_table;
mod m20250210_030348_stream_table;
mod m20250210_030628_stream_message_table;
mod m20250210_033251_emote_table;
mod m20250210_033303_stream_message_emote_table;
mod m20250210_034946_stream_name_table;
mod m20250210_035251_donation_event_table;
mod m20250210_043325_subscription_event_table;
mod m20250210_204036_user_timeout_table;
mod m20250212_180158_add_sub_tier_column_to_donation_event_table;
mod m20250212_201344_add_sub_tier_column_to_subscription_event_table;
mod m20250213_174031_add_third_party_emotes_used_column_to_stream_message_table;
mod m20250213_193501_update_stream_message_table_stream_id_to_be_nullable;
mod m20250215_052013_add_is_subscriber_column_to_stream_message_table;
mod m20250218_202933_add_twitch_emote_usage_column_to_stream_message_table;
mod m20250224_065519_add_raid_table;
mod m20250309_012925_update_donation_event_to_have_optional_name_field;
mod m20250310_202954_create_twitch_user_unknown_user_association_table;
mod m20250402_182948_create_user_name_change_table;
mod m20250419_164430_change_stream_message_content_to_utf8mb4;
mod m20250419_220750_add_twitch_origin_id_column_to_donation_event_table;
mod m20250514_231010_change_stream_message_emote_columns_to_json_from_strings;
mod m20250516_214316_create_bridge_table_for_gift_sub_donation_receivers;
mod m20250705_165646_add_unique_id_column_to_messages;
mod m20250713_194533_add_message_source_id_to_sharedchat_tables;
mod m20250721_001104_update_emote_table_for_third_party_emote_storage;
mod m20250721_001110_convert_stream_message_emote_columns_to_many_to_many_tables;
mod m20251109_005842_add_additional_stream_table_data;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
            Box::new(m20250210_025922_twitch_user_table::Migration),
            Box::new(m20250210_030348_stream_table::Migration),
            Box::new(m20250210_030628_stream_message_table::Migration),
            Box::new(m20250210_033251_emote_table::Migration),
            Box::new(m20250210_033303_stream_message_emote_table::Migration),
            Box::new(m20250210_034946_stream_name_table::Migration),
            Box::new(m20250210_035251_donation_event_table::Migration),
            Box::new(m20250210_043325_subscription_event_table::Migration),
            Box::new(m20250210_204036_user_timeout_table::Migration),
            Box::new(m20250212_180158_add_sub_tier_column_to_donation_event_table::Migration),
            Box::new(m20250212_201344_add_sub_tier_column_to_subscription_event_table::Migration),
            Box::new(m20250213_174031_add_third_party_emotes_used_column_to_stream_message_table::Migration),
            Box::new(m20250213_193501_update_stream_message_table_stream_id_to_be_nullable::Migration),
            Box::new(m20250215_052013_add_is_subscriber_column_to_stream_message_table::Migration),
            Box::new(m20250218_202933_add_twitch_emote_usage_column_to_stream_message_table::Migration),
            Box::new(m20250224_065519_add_raid_table::Migration),
            Box::new(m20250309_012925_update_donation_event_to_have_optional_name_field::Migration),
            Box::new(m20250310_202954_create_twitch_user_unknown_user_association_table::Migration),
            Box::new(m20250402_182948_create_user_name_change_table::Migration),
            Box::new(m20250419_164430_change_stream_message_content_to_utf8mb4::Migration),
            Box::new(m20250419_220750_add_twitch_origin_id_column_to_donation_event_table::Migration),
            Box::new(m20250514_231010_change_stream_message_emote_columns_to_json_from_strings::Migration),
            Box::new(m20250516_214316_create_bridge_table_for_gift_sub_donation_receivers::Migration),
            Box::new(m20250705_165646_add_unique_id_column_to_messages::Migration),
            Box::new(m20250713_194533_add_message_source_id_to_sharedchat_tables::Migration),
            Box::new(m20250721_001104_update_emote_table_for_third_party_emote_storage::Migration),
            Box::new(m20250721_001110_convert_stream_message_emote_columns_to_many_to_many_tables::Migration),
            Box::new(m20251109_005842_add_additional_stream_table_data::Migration),
        ]
  }
}
