import { CategoryState } from "./CategoryState";
import { AdditionalFilter } from "./AdditionalFilters";

export interface QueryFormData {
  category: CategoryState;
  channelSearchQuery: string;
  userSearchQuery: string;
  messageSearch: string;
  additionalFilters: AdditionalFilter[];
  // Messages-only: switches to paginating a whole month at a time instead of by page.
  paginateByMonth: boolean;
}
