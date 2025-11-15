import { Response } from "./Response";
import { Stream } from "./Streams";
import { UnknownUser, User } from "./users";

export interface Donation {
  id: number;
  event_type: EventType;
  amount: number;
  timestamp: string;
  donator: User | null;
  donation_receiver: User;
  stream: Stream | null;
  subscription_tier: number | null;
  unknown_user: UnknownUser | null;
}

export enum EventType {
  StreamlabsDonation = "StreamlabsDonation",
  GiftSubs = "GiftSubs",
  Bits = "Bits",
}

export interface DonationResponse extends Response<Donation[]> {}
