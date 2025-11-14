import { useEffect, useState } from 'react';
import { MutedVodSegment } from '../types/Streams';

export interface MutedVodTableProps {
  muted_vod_segments: MutedVodSegment[],
}

export default function MutedVodTable({
  muted_vod_segments
}: MutedVodTableProps) {
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = 'unset';
    }

    return () => {
      document.body.style.overflow = 'unset';
    };
  }, [isOpen]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isOpen]);

  return (
    <>
      <button
        onClick={() => setIsOpen(true)}
        className="text-red-500 hover:underline"
      >
        Muted
      </button>

      {isOpen && (
        <>
          <div
            className="fixed inset-0 bg-black opacity-50"
            onClick={() => setIsOpen(false)}
          />
          <div className="fixed inset-0 flex items-center justify-center p-4 pointer-events-none">
            <div className="rounded-xl shadow-2xl max-w-2xl w-full bg-gray-900 border border-gray-800 overflow-hidden pointer-events-auto max-h-[80vh] flex flex-col">
              <div className="p-6 flex-shrink-0">
                <div className="flex justify-between items-center">
                  <h2 className="text-xl font-semibold text-gray-300">Muted VOD Segments</h2>
                  <button
                    onClick={() => setIsOpen(false)}
                    className="text-gray-500 hover:text-gray-300"
                  >
                    ✕
                  </button>
                </div>
              </div>

              <div className="overflow-y-auto flex-1">
                <table className="w-full">
                  <thead className="bg-gray-800 border-b border-gray-700 sticky top-0">
                    <tr>
                      <th className="text-left px-6 py-4 text-sm font-semibold text-gray-300 uppercase tracking-wider">Offset</th>
                      <th className="text-left px-6 py-4 text-sm font-semibold text-gray-300 uppercase tracking-wider">Duration (seconds)</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-800">
                    {muted_vod_segments.map((segment, index) => (
                      <tr key={index} className="hover:bg-gray-800/50 transition-colors">
                        <td className="px-6 py-4 text-sm text-gray-300">{segment.start}</td>
                        <td className="px-6 py-4 text-sm text-gray-300">{segment.duration}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </>
      )}
    </>
  );
}
