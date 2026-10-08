import { User } from "./users";

export interface UserMessageResponse {
  user: User;
  channel: User;

  messages: UserMessage[]

  // The distinct `YYYY-MM` months this user has messages for. Only populated when
  // the request included `per_month=true`.
  available_months: string[]
}

export interface UserMessage {
  id: number,
  is_first_message: boolean,
  timestamp: string,
  contents: string,
  is_subscriber: boolean,
  emote_usage: Emote[],
}

export interface Emote {
  contents_indices: number[],
  emote_name_size: number,
  emote_image_url: string,
}
