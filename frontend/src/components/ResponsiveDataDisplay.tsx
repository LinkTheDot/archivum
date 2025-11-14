export interface Column<T> {
  header_name: string;
  header_value_key?: keyof T;
  render?: (item: T) => React.ReactNode;
  mobileLabel?: string;
}

export interface ResponsiveDataDisplayProps<T> {
  data: T[];
  columns: Column<T>[];
  rowKey: keyof T | ((item: T) => string | number);
  emptyMessage?: string;
}

export function ResponsiveDataDisplay<T>({
  data,
  columns,
  rowKey,
  emptyMessage = "No results found."
}: ResponsiveDataDisplayProps<T>) {
  console.log("Got to ResponsiveDataDisplay");

  const renderValue = (value: React.ReactNode) => {
    if (value === null || value === undefined) {
      return <span className="text-gray-500">Unknown</span>;
    }
    return value;
  };

  if (!data || data.length === 0) {
    return (
      <div className="text-center py-12 text-gray-400">
        <p className="text-lg">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <>
      <div className="md:hidden space-y-4">
        {data.map((item) => {
          const rowKeyValue = typeof rowKey === 'function' ? rowKey(item) : item[rowKey] as string | number;

          return (
            <div key={rowKeyValue} className="bg-gray-900 rounded-lg border border-gray-800 p-4 space-y-3">
              {columns.map((column, colIndex) => (
                <div key={`${rowKeyValue}-${column.header_name || colIndex}`} className="flex flex-col space-y-1">
                  <span className="text-xs font-medium text-gray-500 uppercase tracking-wider">
                    {column.mobileLabel || column.header_name}
                  </span>
                  <span className="text-sm text-gray-300">
                    {renderValue(column.render ? column.render(item) : (column.header_value_key ? item[column.header_value_key] as React.ReactNode : ''))}
                  </span>
                </div>
              ))}
            </div>
          );
        })}
      </div>

      <div className="hidden md:grid lg:hidden grid-cols-1 gap-4">
        {data.map((item) => {
          const rowKeyValue = typeof rowKey === 'function' ? rowKey(item) : item[rowKey] as string | number;

          return (
            <div key={rowKeyValue} className="bg-gray-900 rounded-lg border border-gray-800 p-6">
              <div className="grid grid-cols-2 gap-4">
                {columns.map((column, colIndex) => (
                  <div key={`${rowKeyValue}-${column.header_name || colIndex}`}>
                    <p className="text-xs font-medium text-gray-500 uppercase tracking-wider mb-1">
                      {column.header_name}
                    </p>
                    <p className="text-sm text-gray-300">
                      {renderValue(column.render ? column.render(item) : (column.header_value_key ? item[column.header_value_key] as React.ReactNode : ''))}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>

      <div className="hidden lg:block bg-gray-900 rounded-xl shadow-2xl border border-gray-800 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-800 border-b border-gray-700">
              <tr>
                {columns.map((column, index) => (
                  <th key={column.header_name || index} className="px-6 py-4 text-left text-sm font-semibold text-gray-300 uppercase tracking-wider">
                    {column.header_name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800">
              {data.map((item) => {
                const rowKeyValue = typeof rowKey === 'function' ? rowKey(item) : item[rowKey] as string | number;

                return (
                  <tr key={rowKeyValue} className="hover:bg-gray-800/50 transition-colors">
                    {columns.map((column, colIndex) => (
                      <td key={`${rowKeyValue}-${column.header_name || colIndex}`} className="px-6 py-4 text-sm text-gray-300">
                        {renderValue(column.render ? column.render(item) : (column.header_value_key ? item[column.header_value_key] as React.ReactNode : ''))}
                      </td>
                    ))}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
