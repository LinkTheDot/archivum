import { Pagination } from "../types/Pagination"

export interface FetchUrlProps {
  route: string;
  dataName: string;
  data: string | null;
  pagination: Pagination | null;
  channel?: string | null;
  additional?: string | null;
}

export const buildFetchUrl = (props: FetchUrlProps): string => {
  const route = props.channel ? `/${props.channel}${props.route}` : props.route
  const fetchUrl = `${import.meta.env.VITE_BACKEND_URL}${route}`
  const fetchData = props.data ? `?${props.dataName}=${props.data}` : "";

  const pageJoiner = fetchData ? '&' : '?';
  const page = props.pagination ? `${pageJoiner}page=${props.pagination.page}` : "";
  const additionalDataJoiner = fetchData || page ? '&' : '?';
  const additionalData = props.additional ? `${additionalDataJoiner}${props.additional}` : "";

  return `${fetchUrl}${fetchData}${page}${additionalData}`;
}
