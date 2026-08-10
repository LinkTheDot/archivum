// Stackable, ad-hoc filters for the user messages query.
// Each entry maps onto one or more backend query parameters (see
// backend/src/routes/users/messages.rs -> UserMessagesQuery).
export enum FilterField {
  DateStart = "date_start",
  DateEnd = "date_end",
  During = "during",
  StreamId = "stream_id",
}

export interface FilterFieldConfig {
  label: string;
  inputType: "number" | "text";
  // Hints the on-screen keyboard on mobile without pulling in a spinner (unlike
  // inputType: "number", which does that regardless of inputMode).
  inputMode?: "numeric";
  placeholder?: string;
  // Fields that can't be active at the same time as this one.
  conflictsWith?: FilterField[];
  // Converts the raw form input into the already-encoded `key=value[&key=value]`
  // query string segment(s) sent to the backend. Return "" if the input is invalid.
  toQueryValue: (rawValue: string) => string;
  // Converts the raw form input into a human readable value for the filter chip.
  toDisplayValue: (rawValue: string) => string;
}

// Matches "YYYY", "YYYY-MM", "YYYY-MM-DD", or a full date-time ("YYYY-MM-DD[T ]HH:mm[:ss]").
// Every unit past the year is optional, but each one requires everything before it.
const PARTIAL_DATETIME_PATTERN =
  /^(\d{4})(?:-(\d{2})(?:-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?)?)?)?$/;

interface PartialDateTime {
  year: number;
  month?: number;
  day?: number;
  hour?: number;
  minute?: number;
  second?: number;
}

const isValidCalendarDate = (year: number, month: number, day: number): boolean => {
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  );
};

// Backend's `date_start`/`date_end` params parse via `DateTime<Utc>`'s FromStr (RFC 3339), so
// the front-end just needs to build that string for whatever granularity was given.
const parsePartialDateTime = (rawValue: string): PartialDateTime | null => {
  const match = PARTIAL_DATETIME_PATTERN.exec(rawValue.trim());
  if (!match) {
    return null;
  }

  const [, yearStr, monthStr, dayStr, hourStr, minuteStr, secondStr] = match;
  const year = Number(yearStr);
  const month = monthStr ? Number(monthStr) : undefined;
  const day = dayStr ? Number(dayStr) : undefined;
  const hour = hourStr ? Number(hourStr) : undefined;
  const minute = minuteStr ? Number(minuteStr) : undefined;
  const second = secondStr ? Number(secondStr) : undefined;

  if (month !== undefined && (month < 1 || month > 12)) {
    return null;
  }

  if (day !== undefined && month !== undefined && !isValidCalendarDate(year, month, day)) {
    return null;
  }

  if (hour !== undefined && (hour > 23 || (minute ?? 0) > 59 || (second ?? 0) > 59)) {
    return null;
  }

  return { year, month, day, hour, minute, second };
};

// Resolves a partial date/time to a concrete instant. When a unit is left unspecified,
// `boundary` decides whether it's rounded down to the start of that period ("floor",
// for an "after"/date_start filter) or up to its end ("ceil", for a "before"/date_end
// filter). A fully-specified time is always treated as an exact instant.
const toBoundaryDate = (rawValue: string, boundary: "floor" | "ceil"): Date | null => {
  const parsed = parsePartialDateTime(rawValue);
  if (!parsed) {
    return null;
  }

  const { year, month, day, hour, minute, second } = parsed;

  if (hour !== undefined) {
    return new Date(Date.UTC(year, (month ?? 1) - 1, day ?? 1, hour, minute ?? 0, second ?? 0));
  }

  if (boundary === "floor") {
    return new Date(Date.UTC(year, (month ?? 1) - 1, day ?? 1));
  }

  // "ceil": roll up to the end of whatever the smallest given unit is, then pull back
  // 1ms from the start of the next one so the range doesn't spill into it.
  let exclusiveEnd: Date;
  if (day !== undefined) {
    exclusiveEnd = new Date(Date.UTC(year, (month as number) - 1, day + 1));
  } else if (month !== undefined) {
    exclusiveEnd = new Date(Date.UTC(year, month, 1));
  } else {
    exclusiveEnd = new Date(Date.UTC(year + 1, 0, 1));
  }

  return new Date(exclusiveEnd.getTime() - 1);
};

// Same parsing as the before/after filters, but day and time aren't accepted — "during"
// only ever spans a whole year or a whole month.
const parseDuringRange = (rawValue: string): { start: Date; end: Date } | null => {
  const parsed = parsePartialDateTime(rawValue);
  if (!parsed || parsed.day !== undefined) {
    return null;
  }

  const start = toBoundaryDate(rawValue, "floor");
  const end = toBoundaryDate(rawValue, "ceil");

  return start && end ? { start, end } : null;
};

const DATE_PLACEHOLDER = "YYYY, YYYY-MM, YYYY-MM-DD, or YYYY-MM-DDTHH:mm";

export const FILTER_FIELD_CONFIG: Record<FilterField, FilterFieldConfig> = {
  [FilterField.DateStart]: {
    label: "Date Start",
    inputType: "text",
    placeholder: DATE_PLACEHOLDER,
    conflictsWith: [FilterField.During],
    toQueryValue: (rawValue) => {
      const date = toBoundaryDate(rawValue, "floor");
      return date ? `date_start=${encodeURIComponent(date.toISOString())}` : "";
    },
    toDisplayValue: (rawValue) => rawValue.trim(),
  },
  [FilterField.DateEnd]: {
    label: "Date End",
    inputType: "text",
    placeholder: DATE_PLACEHOLDER,
    conflictsWith: [FilterField.During],
    toQueryValue: (rawValue) => {
      const date = toBoundaryDate(rawValue, "ceil");
      return date ? `date_end=${encodeURIComponent(date.toISOString())}` : "";
    },
    toDisplayValue: (rawValue) => rawValue.trim(),
  },
  [FilterField.During]: {
    label: "During",
    inputType: "text",
    placeholder: "YYYY or YYYY-MM",
    conflictsWith: [FilterField.DateStart, FilterField.DateEnd],
    toQueryValue: (rawValue) => {
      const range = parseDuringRange(rawValue);
      if (!range) {
        return "";
      }

      return `date_start=${encodeURIComponent(range.start.toISOString())}&date_end=${encodeURIComponent(range.end.toISOString())}`;
    },
    toDisplayValue: (rawValue) => rawValue.trim(),
  },
  [FilterField.StreamId]: {
    label: "Stream ID",
    inputType: "text",
    inputMode: "numeric",
    placeholder: "e.g. 40123456789",
    toQueryValue: (rawValue) => {
      const trimmed = rawValue.trim();
      return /^\d+$/.test(trimmed) ? `stream_id=${encodeURIComponent(trimmed)}` : "";
    },
    toDisplayValue: (rawValue) => rawValue.trim(),
  },
};

export interface AdditionalFilter {
  id: string;
  field: FilterField;
  displayValue: string;
  queryValue: string;
}

export const buildAdditionalFiltersQuery = (filters: AdditionalFilter[]): string =>
  filters.map((filter) => filter.queryValue).join("&");

// Builds the `date_start`/`date_end` query segment covering a whole `YYYY-MM` month —
// used by the "paginate by month" mode once a month has been picked from the dropdown.
export const buildMonthQuery = (month: string): string => {
  const start = toBoundaryDate(month, "floor");
  const end = toBoundaryDate(month, "ceil");

  if (!start || !end) {
    return "";
  }

  return `date_start=${encodeURIComponent(start.toISOString())}&date_end=${encodeURIComponent(end.toISOString())}`;
};
