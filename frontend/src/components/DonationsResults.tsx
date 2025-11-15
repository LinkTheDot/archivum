import { useGetData } from "../services/DataRequest";
import { buildFetchUrl } from "../services/FetchUrl";
import { formatDate } from "../services/FormatDate";
import { Donation, EventType } from "../types/Donations";
import { Pagination } from "../types/Pagination";
import { QueryFormData } from "../types/QueryFormData";
import { Column, ResponsiveDataDisplay } from "./ResponsiveDataDisplay";

export interface DonationsResultsProps {
  queryResults: QueryFormData;
  pagination: Pagination | null;
  updatePagination: (paginationResponse: Pagination | null) => void;
  setIsLoading: (isLoading: boolean) => void;
}

export function DonationsResults(props: DonationsResultsProps) {
  if (!props.queryResults.userSearchQuery && !props.queryResults.channelSearchQuery) {
    return;
  }

  const userIdentifier = props.queryResults.userSearchQuery;
  const requestType = Number(userIdentifier) ? "user_id" : "maybe_login";

  const requestUrl = buildFetchUrl({
    route: "/donations",
    dataName: requestType,
    data: userIdentifier,
    pagination: props.pagination,
    channel: props.queryResults.channelSearchQuery,
  });

  const { response_data, error } = useGetData<Donation[]>({
    requestUrl,
    updatePagination: props.updatePagination,
    setIsLoading: props.setIsLoading,
  });

  const raidColumns: Column<Donation>[] = [
    { header_name: 'Id', header_value_key: 'id' },
    {
      header_name: 'Timestamp',
      render: (item) => (
        <span className="text-sm text-gray-300">
          {formatDate(item.timestamp)}
        </span>
      )
    },
    {
      header_name: 'Donation Type',
      render: (item) => {
        const tier = item.subscription_tier ?? 'Unknown';
        const title = item.event_type === EventType.GiftSubs ? `Tier ${tier}` : undefined;
        return <span title={title}>{item.event_type}</span>;
      }
    },
    {
      header_name: 'Amount',
      render: (item) => {
        let color;
        if (item.subscription_tier === 2) {
          color = 'red';
        } else if (item.subscription_tier === 3) {
          color = '#6495ED';
        }
        return <span style={color ? { color } : undefined}>{item.amount}</span>;
      }
    },
    {
      header_name: 'Donator Name',
      render: (item) => {
        const name = item.donator?.login_name ?? item.unknown_user?.name ?? null;
        if (item.unknown_user?.name) {
          return <span style={{ color: 'red' }} title="Unknown user">{name}</span>;
        }
        return name;
      }
    },
    {
      header_name: 'Channel Name',
      render: (item) => item.donation_receiver?.login_name
    },
  ];

  if (error) {
    return (
      <div className="bg-red-900/20 border border-red-800 rounded-lg p-6 text-center">
        <p className="text-red-400">Error: {error.message || "Failed to fetch users."}</p>
      </div>
    );
  }

  return (
    <>
      {response_data?.data && (
        <ResponsiveDataDisplay
          data={response_data.data}
          columns={raidColumns}
          rowKey="id"
          emptyMessage="No donations found."
        />
      )}
    </>
  );
}
