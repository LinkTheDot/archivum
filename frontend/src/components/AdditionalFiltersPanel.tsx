import { useState } from "react";
import {
  AdditionalFilter,
  FILTER_FIELD_CONFIG,
  FilterField,
} from "../types/AdditionalFilters";

interface AdditionalFiltersPanelProps {
  filters: AdditionalFilter[];
  availableFields: FilterField[];
  onChange: (filters: AdditionalFilter[]) => void;
}

export function AdditionalFiltersPanel({ filters, availableFields, onChange }: AdditionalFiltersPanelProps) {
  const usedFields = filters.map((filter) => filter.field);
  const remainingFields = availableFields.filter((field) => {
    if (usedFields.includes(field)) {
      return false;
    }

    const conflicts = FILTER_FIELD_CONFIG[field].conflictsWith ?? [];
    return !usedFields.some((used) => conflicts.includes(used));
  });

  const [selectedField, setSelectedField] = useState<FilterField | "">(remainingFields[0] ?? "");
  const [inputValue, setInputValue] = useState<string>("");

  // Keep the selected field valid as filters get added/removed.
  const activeSelectedField = remainingFields.includes(selectedField as FilterField)
    ? (selectedField as FilterField)
    : remainingFields[0] ?? "";

  const handleFieldChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedField(event.target.value as FilterField);
    setInputValue("");
  };

  const handleAddFilter = (event: React.SyntheticEvent) => {
    event.preventDefault();

    if (!activeSelectedField || !inputValue.trim()) {
      return;
    }

    const config = FILTER_FIELD_CONFIG[activeSelectedField];

    let queryValue: string;
    let displayValue: string;

    try {
      queryValue = config.toQueryValue(inputValue);
      displayValue = config.toDisplayValue(inputValue);
    } catch {
      return;
    }

    if (!queryValue) {
      return;
    }

    const newFilter: AdditionalFilter = {
      id: `${activeSelectedField}-${Date.now()}`,
      field: activeSelectedField,
      displayValue,
      queryValue,
    };

    onChange([...filters, newFilter]);
    setInputValue("");
  };

  const handleRemoveFilter = (id: string) => {
    onChange(filters.filter((filter) => filter.id !== id));
  };

  const handleClearAll = () => {
    onChange([]);
  };

  const currentConfig = activeSelectedField ? FILTER_FIELD_CONFIG[activeSelectedField] : null;

  return (
    <div className="bg-gray-900 rounded-xl p-4 shadow-2xl border border-gray-800 md:w-72 shrink-0">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-gray-300">Additional Filters</h3>
        {filters.length > 0 && (
          <button
            type="button"
            onClick={handleClearAll}
            className="text-xs text-gray-400 hover:text-red-400 transition-colors"
          >
            Clear all
          </button>
        )}
      </div>

      {remainingFields.length > 0 ? (
        <div className="flex flex-col gap-2 mb-3">
          <select
            value={activeSelectedField}
            onChange={handleFieldChange}
            className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent cursor-pointer"
          >
            {remainingFields.map((field) => (
              <option key={field} value={field} className="bg-gray-800">
                {FILTER_FIELD_CONFIG[field].label}
              </option>
            ))}
          </select>

          <div className="flex gap-2">
            <input
              type={currentConfig?.inputType ?? "text"}
              inputMode={currentConfig?.inputMode}
              value={inputValue}
              placeholder={currentConfig?.placeholder}
              onChange={(event) => setInputValue(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  handleAddFilter(event);
                }
              }}
              className="flex-1 min-w-0 px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent placeholder-gray-500"
            />
            <button
              type="button"
              onClick={handleAddFilter}
              disabled={!inputValue.trim()}
              className="px-3 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors"
            >
              Add
            </button>
          </div>
        </div>
      ) : (
        <p className="text-xs text-gray-500 mb-3">All available filters have been added.</p>
      )}

      {filters.length > 0 ? (
        <ul className="flex flex-col gap-2">
          {filters.map((filter) => (
            <li
              key={filter.id}
              className="flex items-center justify-between gap-2 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2"
            >
              <span className="text-xs text-gray-300 truncate">
                <span className="text-gray-500">{FILTER_FIELD_CONFIG[filter.field].label}:</span>{" "}
                {filter.displayValue}
              </span>
              <button
                type="button"
                onClick={() => handleRemoveFilter(filter.id)}
                aria-label={`Remove ${FILTER_FIELD_CONFIG[filter.field].label} filter`}
                className="shrink-0 text-gray-400 hover:text-red-400 transition-colors"
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-xs text-gray-500">No additional filters applied.</p>
      )}
    </div>
  );
}
