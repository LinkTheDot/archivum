import {Response} from './Response';

export interface User {
  id: number;
  twitch_id: number;
  display_name: string;
  login_name: string;
}

export interface UnknownUser {
  id: number;
  name: string;
  created_at: string;
}

export interface UserResponse extends Response<User[]> {}
