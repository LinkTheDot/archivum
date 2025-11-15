import { useGetData } from "../services/DataRequest";
import { buildFetchUrl } from "../services/FetchUrl";
import { formatDate } from "../services/FormatDate";
import { Pagination } from "../types/Pagination";
import { QueryFormData } from "../types/QueryFormData";
import { Raid, Raids } from "../types/Raid";
import { Column, ResponsiveDataDisplay } from "./ResponsiveDataDisplay";

export interface RaidsResultsProps {
  queryResults: QueryFormData;
  pagination: Pagination | null;
  updatePagination: (paginationResponse: Pagination | null) => void;
  setIsLoading: (isLoading: boolean) => void;
}

export function RaidsResults(props: RaidsResultsProps) {
  if (!props.queryResults.userSearchQuery && !props.queryResults.channelSearchQuery) {
    return;
  }

  const channelIdentifier = props.queryResults.channelSearchQuery;
  const raiderIdentifier = props.queryResults.userSearchQuery;

  const channelRequestType = Number(channelIdentifier) ? "channel_id" : "channel_login";
  const raiderRequestType = Number(channelIdentifier) ? "raider_id" : "raider_login";

  const raiderConstraint = props.queryResults.userSearchQuery ? `&${raiderRequestType}=${raiderIdentifier}` : null;

  const requestUrl = buildFetchUrl({
    route: "/users/raids",
    dataName: channelRequestType,
    data: channelIdentifier,
    pagination: props.pagination,
    additional: raiderConstraint,
  });

  const { response_data, error } = useGetData<Raids>({
    requestUrl,
    updatePagination: props.updatePagination,
    setIsLoading: props.setIsLoading,
  });

  const raidColumns: Column<Raid>[] = [
    { header_name: 'Id', header_value_key: 'id' },
    { 
      header_name: 'Raider Name', 
      render: (item) => item.raider?.login_name
    },
    {
      header_name: 'Timestamp',
      render: (item) => (
        <span className="text-sm text-gray-300">
          {formatDate(item.timestamp)}
        </span>
      )
    },
    { header_name: 'Raid Size', header_value_key: 'viewers_from_raid' },
    {
      header_name: 'Stream Title',
      render: (item) => {
        if (!item.stream_title) {
          return null;
        }
        return (
          <div className="max-w-xs truncate" title={item.stream_title}>
            {item.stream_title}
          </div>
        );
      }
    }
  ];

  if (error) {
    return (
      <div className="bg-red-900/20 border border-red-800 rounded-lg p-6 text-center">
        <p className="text-red-400">Error: {error.message || "Failed to fetch users."}</p>
      </div>
    );
  }

  const userName = response_data?.data.channel?.login_name ?? channelIdentifier;

  return (
    <>
      <h3 className="text-center text-xl font-semibold text-gray-200 mb-4">Raid list for `{userName}`</h3>

      {response_data?.data && (
        <ResponsiveDataDisplay
          data={response_data.data.raids}
          columns={raidColumns}
          rowKey="id"
          emptyMessage="No streams found."
        />
      )}
    </>
  );
}
