import { Response } from "./Response";
import { User } from "./users";

export interface Raids {
  channel: User;
  raids: Raid[];
}

export interface Raid {
  id: number;
  raider: User | null;
  timestamp: string;
  viewers_from_raid: number;
  stream_title: string | null;
}

export interface RaidResponse extends Response<Raid[]> {}
