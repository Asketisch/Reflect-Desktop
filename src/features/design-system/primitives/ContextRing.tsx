/**
 * ContextRing —— 上下文用量圆环(SVG)。
 *
 * 5 个分类 segment:tools / system / skills / messages / other
 */
import { ringGeometry, segmentArc } from '../utils/ring';
import type { RingSegment } from '../utils/ring';

export interface ContextRingProps {
  segments: RingSegment[];
  size?: number;
  strokeWidth?: number;
  label?: string;
}

export function ContextRing({ segments, size = 40, strokeWidth = 4, label }: ContextRingProps) {
  const geom = ringGeometry(size, strokeWidth);
  let cursor = 0;
  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
      <svg width={size} height={size} role="img" aria-label={label ?? 'context ring'}>
        {/* Background track */}
        <circle
          cx={geom.cx}
          cy={geom.cy}
          r={geom.radius}
          fill="none"
          stroke="#e2e8f0"
          strokeWidth={strokeWidth}
        />
        {/* Foreground segments */}
        {segments.map((seg, i) => {
          const arc = segmentArc(geom, cursor, Math.max(0, Math.min(1, seg.value)));
          cursor += seg.value;
          return (
            <circle
              key={i}
              cx={geom.cx}
              cy={geom.cy}
              r={geom.radius}
              fill="none"
              stroke={seg.color}
              strokeWidth={strokeWidth}
              strokeDasharray={arc.dasharray}
              strokeDashoffset={arc.dashoffset}
              transform={`rotate(-90 ${geom.cx} ${geom.cy})`}
            />
          );
        })}
      </svg>
      {label && <span style={{ fontSize: 12, color: '#64748b' }}>{label}</span>}
    </div>
  );
}