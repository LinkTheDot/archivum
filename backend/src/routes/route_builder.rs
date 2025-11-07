use crate::app::InterfaceConfig;
use axum::routing::get;

pub trait RouteBuilder {
  fn apply_all_routes(self) -> Self;
  fn apply_user_routes(self) -> Self;
  fn apply_donation_routes(self) -> Self;
}

impl RouteBuilder for axum::Router<InterfaceConfig> {
  fn apply_all_routes(self) -> Self {
    self //
      .apply_user_routes()
      .apply_donation_routes()
  }

  fn apply_user_routes(self) -> Self {
    self
      .route("/users", get(crate::routes::users::get_users::get_users))
      .route(
        "/users/name_changes",
        get(crate::routes::users::name_changes::get_name_changes),
      )
      .route(
        "/users/following",
        get(crate::routes::users::following::get_following),
      )
      .route(
        "/{channel}/users/messages",
        get(crate::routes::users::messages::get_messages),
      )
      .route(
        "/users/streams",
        get(crate::routes::users::streams::get_streams),
      )
  }

  fn apply_donation_routes(self) -> Self {
    self
      .route(
        "/{channel}/donations/subscriptions",
        get(crate::routes::donations::subscriptions::get_subscriptions),
      )
      .route(
        "/donations/subscriptions",
        get(crate::routes::donations::subscriptions::get_subscriptions),
      )
      .route(
        "/{channel}/donations/",
        get(crate::routes::donations::donation_event::get_donations),
      )
      .route(
        "/donations/",
        get(crate::routes::donations::donation_event::get_donations),
      )
    // .route(
    //   "/subathon",
    //   get(crate::routes::donations::subathon_data::get_subathon_data),
    // )
  }
}
