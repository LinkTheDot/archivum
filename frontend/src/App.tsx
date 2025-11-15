import { useState } from 'react'
import './App.css'
import { CategoryState } from './types/CategoryState';
import QueryForm from './components/QueryForm';
import { UserResults } from './components/UserResults';
import { NameChangeResults } from './components/NameChangesResults';
import { SubscriptionResults } from './components/SubscriptionResults';
import { FollowingResults } from './components/FollowingResults';
import { ReturnToTopButton } from './components/ReturnToTopButton';
import { PageSelect } from './components/PageSelect';
import { QueryFormData } from './types/QueryFormData';
import { Pagination } from './types/Pagination';
import { MessageResults } from './components/UserMessageResults';
import { StreamsResults } from './components/StreamsResults';
import { DonationsResults } from './components/DonationsResults'
import { RaidsResults } from './components/RaidsResults';

const CATEGORY_COMPONENTS = {
  [CategoryState.Users]: UserResults,
  [CategoryState.NameChanges]: NameChangeResults,
  [CategoryState.Subscriptions]: SubscriptionResults,
  [CategoryState.Following]: FollowingResults,
  [CategoryState.Messages]: MessageResults,
  [CategoryState.Streams]: StreamsResults,
  [CategoryState.Raids]: RaidsResults,
  [CategoryState.Donations]: DonationsResults,
} as const;

export default function App() {
  const [queryFormData, setQueryForm] = useState<QueryFormData>({
    category: CategoryState.Users,
    channelSearchQuery: "",
    userSearchQuery: "",
    messageSearch: "",
  });
  const [pagination, setPagination] = useState<Pagination | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const updateIsLoading = (isLoadingChange: boolean): void => {
    if (isLoadingChange) {
      setIsLoading(true);
    } else {
      // Delay setting to false by 100ms to avoid page cycling bug.
      setTimeout(() => {
        setIsLoading(false);
      }, 100);
    }
  };

  const updatePagination = (paginationChange: Pagination | null): void => {
    console.log(`Updating pagination: totalItems=${paginationChange?.totalItems} totalPages=${paginationChange?.totalPages} page=${paginationChange?.page} totalSize=${paginationChange?.totalSize}`);

    setPagination(paginationChange);
  };

  const onQueryFormSubmit = (data: QueryFormData): void => {
    setPagination(null);
    setQueryForm(data);
  };

  const ResultComponent = CATEGORY_COMPONENTS[queryFormData.category];

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      <div className="container mx-auto px-4 py-8 lg:px-8 xl:px-12 2xl:px-16">
        <header className="mb-12 text-center">
          <h1 className="text-4xl md:text-5xl font-bold mb-2 bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
            Twitch Chat Tracker
          </h1>
          <p className="text-gray-400 text-lg">Search and analyze Twitch user data</p>
        </header>

        <div className="mb-8">
          <QueryForm onSubmitQuery={onQueryFormSubmit} />
        </div>

        {(isLoading || (pagination && pagination.totalItems > 0 && (queryFormData.userSearchQuery || queryFormData.channelSearchQuery))) && (
          <div className="flex justify-end items-center mb-8 relative">
            {isLoading && (
              <div className="absolute left-1/2 transform -translate-x-1/2 flex items-center">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-500"></div>
                <span className="ml-3 text-gray-400">Loading user messages...</span>
              </div>
            )}

            {pagination && pagination.totalItems > 0 && (queryFormData.userSearchQuery || queryFormData.channelSearchQuery) && (
              <div className="flex items-center gap-4">
                <PageSelect
                  pagination={pagination}
                  onPageChange={updatePagination}
                  isLoading={isLoading}
                />
              </div>
            )}
          </div>
        )}

        <main className="mx-auto">
          {ResultComponent && (
            <ResultComponent
              queryResults={queryFormData}
              pagination={pagination}
              updatePagination={updatePagination}
              setIsLoading={updateIsLoading}
            />
          )}
        </main>

        <div>
          <ReturnToTopButton />
        </div>
      </div>
    </div>
  )
}
