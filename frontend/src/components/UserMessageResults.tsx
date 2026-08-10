import { useEffect, useState } from 'react';
import { formatDateTime } from '../services/FormatDate';
import { buildFetchUrl } from '../services/FetchUrl';
import { useGetData } from '../services/DataRequest';
import { QueryFormData } from '../types/QueryFormData';
import { Pagination } from '../types/Pagination';
import { Emote, UserMessage, UserMessageResponse } from '../types/UserMessage';
import { User } from '../types/users';
import { buildAdditionalFiltersQuery, buildMonthQuery } from '../types/AdditionalFilters';

// Large enough that a `per_month` request comes back with the whole month in one page.
const PER_MONTH_PAGE_SIZE = 1_000_000;

interface MessageResultsProps {
  queryResults: QueryFormData;
  pagination: Pagination | null;
  updatePagination: (paginationResponse: Pagination | null) => void;
  setIsLoading: (isLoading: boolean) => void;
}

// Main component
export function MessageResults(props: MessageResultsProps) {
  if (!props.queryResults.userSearchQuery || !props.queryResults.channelSearchQuery) {
    let missingData;

    if (!props.queryResults.userSearchQuery && !props.queryResults.channelSearchQuery) {
      missingData = "user and channel";
    } else if (!props.queryResults.userSearchQuery) {
      missingData = "user"
    } else {
      missingData = "channel"
    }

    return (
      <div className="bg-red-900/20 border border-red-800 rounded-lg p-6 text-center">
        <p className="text-red-400">Error: {`Missing ${missingData}` || "Failed to fetch users."}</p>
      </div>
    );
  }

  const userIdentifier = props.queryResults.userSearchQuery || props.queryResults.channelSearchQuery;
  const requestType = Number(userIdentifier) ? "user_id" : "maybe_login";
  const paginateByMonth = props.queryResults.paginateByMonth;

  // The month can only be picked once we know what's available, which only comes back
  // on a `per_month` response — so this starts empty and gets filled in below.
  const [selectedMonth, setSelectedMonth] = useState<string>("");

  let additionalData = paginateByMonth && selectedMonth
    ? `page_size=${PER_MONTH_PAGE_SIZE}`
    : "page_size=1000";

  if (props.queryResults.messageSearch) {
    additionalData += `&message_search=${props.queryResults.messageSearch}`;
  } else {
    console.log("No message search found.");
  }

  if (props.queryResults.additionalFilters.length > 0) {
    additionalData += `&${buildAdditionalFiltersQuery(props.queryResults.additionalFilters)}`;
  }

  if (paginateByMonth) {
    additionalData += "&per_month=true";

    if (selectedMonth) {
      additionalData += `&${buildMonthQuery(selectedMonth)}`;
    }
  }

  const requestUrl = buildFetchUrl({
    route: "/users/messages",
    dataName: requestType,
    data: userIdentifier,
    pagination: props.pagination,
    channel: props.queryResults.channelSearchQuery,
    additional: additionalData,
  });

  // Paginate-by-month always fetches a whole month in one uncapped page, so the
  // regular page-number pagination never applies here — suppress it entirely rather
  // than letting it flash on during the initial (not-yet-month-scoped) response.
  const updatePagination = (paginationResponse: Pagination | null) => {
    props.updatePagination(paginateByMonth ? null : paginationResponse);
  };

  const { response_data, error } = useGetData<UserMessageResponse>({
    requestUrl,
    updatePagination,
    setIsLoading: props.setIsLoading
  });

  const availableMonths = response_data?.data.available_months ?? [];
  const availableMonthsKey = availableMonths.join(",");

  // Once the months are known, default to the newest one (or fall back if the
  // previously selected month isn't available for this user/channel anymore).
  useEffect(() => {
    if (paginateByMonth && availableMonths.length > 0 && !availableMonths.includes(selectedMonth)) {
      setSelectedMonth(availableMonths[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paginateByMonth, availableMonthsKey]);

  if (error) {
    return (
      <div className="bg-red-900/20 border border-red-800 rounded-lg p-6 text-center">
        <p className="text-red-400">Error: {error.message || "Failed to fetch user messages."}</p>
      </div>
    );
  }

  // There's a known set of months but the effect above hasn't picked one yet — the
  // messages shown right now aren't month-scoped, so hold off rendering them.
  const isResolvingMonth = paginateByMonth && availableMonths.length > 0 && !selectedMonth;

  return (
    <>
      {response_data?.data && (
        <div className="bg-gray-900/50 rounded-lg border border-gray-700">
          {/* Header */}
          <div className="px-4 py-3 border-b border-gray-700 flex items-center justify-between gap-4 flex-wrap">
            <h2 className="text-lg font-semibold text-gray-100">
              Chat Messages from `{response_data.data.user.display_name}` to `{response_data.data.channel.display_name}`
            </h2>

            {paginateByMonth && availableMonths.length > 0 && selectedMonth && (
              <label className="flex items-center gap-2 text-sm text-gray-400">
                Month:
                <select
                  value={selectedMonth}
                  onChange={(event) => setSelectedMonth(event.target.value)}
                  className="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg text-sm text-gray-100 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent cursor-pointer"
                >
                  {availableMonths.map((month) => (
                    <option key={month} value={month} className="bg-gray-800">
                      {month}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>

          {isResolvingMonth ? (
            <div className="px-4 py-8 text-center">
              <p className="text-gray-400">Loading available months…</p>
            </div>
          ) : (
            <>
              {/* Messages list */}
              <div className="divide-y divide-gray-700/50">
                {response_data.data.messages.map((message) => (
                  <MessageRow key={message.id} message={message} user={response_data.data.user} />
                ))}
              </div>

              {/* Empty state */}
              {response_data.data.messages.length === 0 && (
                <div className="px-4 py-8 text-center">
                  <p className="text-gray-400">No messages found</p>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </>
  );
}

// Component to render a single message with emotes
const MessageRow: React.FC<{ message: UserMessage; user: User }> = ({ message, user }) => {
  const renderMessageWithEmotes = (contents: string, emoteUsage: Emote[]): React.ReactNode => {
    if (!emoteUsage || emoteUsage.length === 0) {
      return contents;
    }

    // Convert string to UTF-8 bytes to match backend indexing
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const contentBytes = encoder.encode(contents);

    // Flatten all emote instances with their positions
    const allEmoteInstances: Array<{
      contents_index: number;
      emote_name_size: number;
      emote_image_url: string;
    }> = [];

    // Process each emote type and add all its instances
    for (const emote of emoteUsage) {
      const { contents_indices, emote_name_size, emote_image_url } = emote;

      // Add each instance of this emote
      for (const index of contents_indices) {
        allEmoteInstances.push({
          contents_index: index,
          emote_name_size,
          emote_image_url
        });
      }
    }

    // Sort all emote instances by their position in the message (ascending)
    const sortedEmotes = allEmoteInstances.sort((a, b) => a.contents_index - b.contents_index);

    // Validate emote positions to prevent out-of-bounds errors
    const validEmotes = sortedEmotes.filter(emote => {
      const endIndex = emote.contents_index + emote.emote_name_size;
      return emote.contents_index >= 0 &&
        emote.contents_index < contentBytes.length &&
        endIndex <= contentBytes.length &&
        emote.emote_name_size > 0;
    });

    const parts: React.ReactNode[] = [];
    let lastByteIndex = 0;
    let keyCounter = 0;

    // Process each emote instance in sorted order
    for (const emote of validEmotes) {
      const { contents_index, emote_name_size, emote_image_url } = emote;
      const endByteIndex = contents_index + emote_name_size;

      // Add text before this emote (convert byte indices back to string)
      if (contents_index > lastByteIndex) {
        const textBeforeBytes = contentBytes.slice(lastByteIndex, contents_index);
        const textBefore = decoder.decode(textBeforeBytes);
        if (textBefore) {
          parts.push(
            <span key={`text-${keyCounter++}`}>{textBefore}</span>
          );
        }
      }

      // Get the emote name (convert byte indices back to string)
      const emoteNameBytes = contentBytes.slice(contents_index, endByteIndex);
      const emoteName = decoder.decode(emoteNameBytes);

      // Add the emote image
      parts.push(
        <img
          key={`emote-${keyCounter++}-${contents_index}`}
          src={emote_image_url}
          alt={emoteName}
          className="inline-block h-6 w-auto mx-0.5"
          style={{ verticalAlign: 'middle' }}
          onError={(e) => {
            console.log(`Emote failed to load: ${emoteName} at ${emote_image_url}`);
            // Replace with text if image fails to load
            e.currentTarget.style.display = 'none';
            const textNode = document.createTextNode(emoteName);
            e.currentTarget.parentNode?.insertBefore(textNode, e.currentTarget);
          }}
        // onLoad={() => console.log(`Emote loaded: ${emoteName}`)}
        />
      );

      lastByteIndex = endByteIndex;
    }

    // Add any remaining text at the end
    if (lastByteIndex < contentBytes.length) {
      const textAfterBytes = contentBytes.slice(lastByteIndex);
      const textAfter = decoder.decode(textAfterBytes);
      if (textAfter) {
        parts.push(
          <span key={`text-${keyCounter++}`}>{textAfter}</span>
        );
      }
    }

    return <span className="inline-flex items-center flex-wrap">{parts}</span>;
  };

  const rowClasses = `
    flex items-center gap-2 py-1 px-2 text-sm
    ${message.is_first_message
      ? 'bg-green-900/30'
      : 'hover:bg-gray-800/50'
    }
  `;

  return (
    <div className={rowClasses}>
      {/* Date */}
      <span className="text-gray-400 text-xs font-mono shrink-0 w-32">
        {formatDateTime(message.timestamp)}
      </span>

      {/* Subscriber badge */}
      {message.is_subscriber && (
        <img
          src="https://static-cdn.jtvnw.net/badges/v1/5d9f2208-5dd8-11e7-8513-2ff4adfae661/3"
          alt="Subscriber"
          className="h-4 w-4 shrink-0"
          onError={(e) => {
            console.log('Badge image failed to load:', e);
            e.currentTarget.style.display = 'none';
          }}
        // onLoad={() => console.log('Badge image loaded successfully')}
        />
      )}

      {/* Username */}
      <span className="text-purple-300 font-medium shrink-0">
        {user.login_name}:
      </span>

      {/* Message content with emotes */}
      <div className="text-gray-100 flex-1 min-w-0">
        {renderMessageWithEmotes(message.contents, message.emote_usage)}
      </div>
    </div>
  );
};
