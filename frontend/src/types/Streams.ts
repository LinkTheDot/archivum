import { User } from "./users";

export interface Streams {
  user: User;
  streams: Stream[];
}

export interface Stream {
  id: number;
  twitch_stream_id: number;
  start_timestamp: string;
  end_timestamp: string;
  twitch_vod_id: string | null;
  title: string | null;
  muted_vod_segments: MutedVodSegment[];
}

export interface MutedVodSegment {
  start: string;
  // In seconds
  duration: number;
}
