#[derive(Debug, Clone)]
pub enum ChannelIdentifier<S: AsRef<str>> {
  Login(S),
  TwitchID(S),
}

impl<'a> ChannelIdentifier<&'a str> {
  pub fn to_owned(&self) -> ChannelIdentifier<String> {
    match self {
      Self::Login(login) => ChannelIdentifier::Login((*login).to_owned()),
      Self::TwitchID(twitch_id) => ChannelIdentifier::TwitchID((*twitch_id).to_owned()),
    }
  }

  pub fn to_str(&self) -> &'a str {
    match self {
      ChannelIdentifier::Login(value) => value,
      ChannelIdentifier::TwitchID(value) => value,
    }
  }

  pub fn from_login_list<I: IntoIterator<Item = &'a str>>(list: I) -> Vec<Self> {
    list.into_iter().map(ChannelIdentifier::Login).collect()
  }
}

impl<'a> From<ChannelIdentifier<&'a str>> for &'a str {
  fn from(value: ChannelIdentifier<&'a str>) -> Self {
    match value {
      ChannelIdentifier::Login(value) => value,
      ChannelIdentifier::TwitchID(value) => value,
    }
  }
}
