import { useState } from "react";
import { CategoryState } from "../types/CategoryState";
import { QueryFormData } from "../types/QueryFormData";
import { AdditionalFilter, FilterField } from "../types/AdditionalFilters";
import { AdditionalFiltersPanel } from "./AdditionalFiltersPanel";

interface QueryFormProps {
  onSubmitQuery: (data: QueryFormData) => void;
}

// Filters made available on the Messages category's side panel.
const MESSAGE_FILTER_FIELDS = [
  FilterField.DateStart,
  FilterField.DateEnd,
  FilterField.During,
  FilterField.StreamId,
];
// "Paginate by month" picks the date range itself, so the manual date filters are
// dropped from the panel (and cleared below) while it's active.
const DATE_FAMILY_FIELDS = [FilterField.DateStart, FilterField.DateEnd, FilterField.During];

const QueryForm: React.FC<QueryFormProps> = ({ onSubmitQuery }) => {
  const [formData, setFormData] = useState<QueryFormData>({
    category: CategoryState.Users,
    channelSearchQuery: '',
    userSearchQuery: '',
    messageSearch: '',
    additionalFilters: [],
    paginateByMonth: false,
  });

  const categoryOptions = Object.values(CategoryState);

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    console.log('Form data being submitted:', formData);
    onSubmitQuery(formData);
  };

  const handleCategoryChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setFormData({ ...formData, category: event.target.value as CategoryState });
  };

  const handleChannelSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData({ ...formData, channelSearchQuery: event.target.value });
  };

  const handleUserSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData({ ...formData, userSearchQuery: event.target.value });
  };

  const handleMessageSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setFormData({ ...formData, messageSearch: event.target.value });
  };

  const handleAdditionalFiltersChange = (additionalFilters: AdditionalFilter[]) => {
    setFormData({ ...formData, additionalFilters });
  };

  const handlePaginateByMonthChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const paginateByMonth = event.target.checked;

    setFormData({
      ...formData,
      paginateByMonth,
      // The month picker owns the date range once enabled, so drop any manual date filters.
      additionalFilters: paginateByMonth
        ? formData.additionalFilters.filter((filter) => !DATE_FAMILY_FIELDS.includes(filter.field))
        : formData.additionalFilters,
    });
  };

  const isMessagesCategory = formData.category === "Messages";
  const isStreamsCategory = formData.category === CategoryState.Streams;
  const isChannelCategory = [
    CategoryState.Subscriptions,
    CategoryState.Messages,
    CategoryState.Raids,
    CategoryState.Donations
  ].includes(formData.category);
  const userIsOptional = [
    CategoryState.Subscriptions,
    CategoryState.Raids,
    CategoryState.Donations,
  ].includes(formData.category);

  // Streams are looked up by their broadcasting channel, not a chatter's username.
  const primaryFieldLabel = isStreamsCategory
    ? "Channel"
    : userIsOptional
      ? "Username / Twitch ID (Optional)"
      : "Username / Twitch ID";
  const primaryFieldPlaceholder = isStreamsCategory ? "Channel" : "Username / Twitch ID";

  return (
    <div className="flex flex-col lg:flex-row gap-4 items-start">
      <form onSubmit={handleSubmit} className="flex-1 w-full bg-gray-900 rounded-xl p-6 shadow-2xl border border-gray-800">
        <div className="hidden md:flex gap-2 mb-2">
          <label htmlFor="username" className="flex-1 text-sm font-medium text-gray-400">
            {primaryFieldLabel}
          </label>
          {isChannelCategory && (
            <label htmlFor="channel" className="flex-1 text-sm font-medium text-gray-400">
              Channel
            </label>
          )}
          {isMessagesCategory && (
            <label htmlFor="message-search" className="flex-1 text-sm font-medium text-gray-400">
              Message Search
            </label>
          )}
          <label htmlFor="search-type" className="flex-1 text-sm font-medium text-gray-400">
            Search Type
          </label>
          <div className="flex-1"></div>
        </div>
        <div className="flex flex-col md:flex-row gap-4">
          <div className="flex-1">
            <label htmlFor="username" className="block md:hidden text-sm font-medium text-gray-400 mb-2">
              {primaryFieldLabel}
            </label>
            <input
              id="username"
              type="text"
              placeholder={primaryFieldPlaceholder}
              value={isStreamsCategory ? formData.channelSearchQuery : formData.userSearchQuery}
              onChange={isStreamsCategory ? handleChannelSearchChange : handleUserSearchChange}
              className="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition-all placeholder-gray-500"
            />
          </div>

          {isChannelCategory && (
            <div className="flex-1">
              <label htmlFor="channel" className="block md:hidden text-sm font-medium text-gray-400 mb-2">
                Channel
              </label>
              <input
                id="channel"
                type="text"
                placeholder="Channel"
                value={formData.channelSearchQuery}
                onChange={handleChannelSearchChange}
                className="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition-all placeholder-gray-500"
              />
            </div>
          )}

          {isMessagesCategory && (
            <div className="flex-1">
              <label htmlFor="message-search" className="block md:hidden text-sm font-medium text-gray-400 mb-2">
                Message Search
              </label>
              <input
                id="message-search"
                type="text"
                placeholder="Search messages..."
                value={formData.messageSearch}
                onChange={handleMessageSearchChange}
                className="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition-all placeholder-gray-500"
              />
            </div>
          )}

          <div className="flex-1 relative">
            <label htmlFor="search-type" className="block md:hidden text-sm font-medium text-gray-400 mb-2">
              Search Type
            </label>
            <select
              id="search-type"
              value={formData.category}
              onChange={handleCategoryChange}
              className="w-full px-4 py-3 pr-10 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition-all cursor-pointer appearance-none"
            >
              {categoryOptions.map((category) => (
                <option key={category} value={category} className="bg-gray-800">
                  {category}
                </option>
              ))}
            </select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-gray-400">
              <svg className="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clipRule="evenodd" />
              </svg>
            </div>
          </div>

          <button
            type="submit"
            className="flex-1 px-6 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 text-white font-semibold rounded-lg shadow-lg transform transition-all duration-200 hover:scale-105 focus:outline-none focus:ring-2 focus:ring-purple-500"
          >
            Search
          </button>
        </div>

        {isMessagesCategory && (
          <div className="flex items-center gap-2 mt-4">
            <input
              id="paginate-by-month"
              type="checkbox"
              checked={formData.paginateByMonth}
              onChange={handlePaginateByMonthChange}
              className="w-4 h-4 rounded border-gray-700 bg-gray-800 text-purple-600 cursor-pointer focus:outline-none focus:ring-2 focus:ring-purple-500"
            />
            <label htmlFor="paginate-by-month" className="text-sm text-gray-400 cursor-pointer">
              Paginate by month
            </label>
          </div>
        )}
      </form>

      {isMessagesCategory && (
        <AdditionalFiltersPanel
          filters={formData.additionalFilters}
          availableFields={formData.paginateByMonth ? [FilterField.StreamId] : MESSAGE_FILTER_FIELDS}
          onChange={handleAdditionalFiltersChange}
        />
      )}
    </div>
  );
};

export default QueryForm;
