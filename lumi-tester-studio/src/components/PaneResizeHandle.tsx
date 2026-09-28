import React, { useCallback } from 'react';
import { clsx } from 'clsx';

interface PaneResizeHandleProps {
  orientation: 'vertical' | 'horizontal';
  label: string;
  reverse?: boolean;
  onResize: (delta: number) => void;
}

export const PaneResizeHandle: React.FC<PaneResizeHandleProps> = ({
  orientation,
  label,
  reverse = false,
  onResize,
}) => {
  const handlePointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();

    let lastPosition = orientation === 'vertical' ? event.clientX : event.clientY;
    const resizeClass = `ide-pane-is-resizing-${orientation}`;
    document.body.classList.add(resizeClass);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const position = orientation === 'vertical' ? moveEvent.clientX : moveEvent.clientY;
      const delta = position - lastPosition;
      lastPosition = position;
      onResize(reverse ? -delta : delta);
    };
    const finishResize = () => {
      document.body.classList.remove(resizeClass);
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finishResize);
      window.removeEventListener('pointercancel', finishResize);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finishResize);
    window.addEventListener('pointercancel', finishResize);
  }, [onResize, orientation, reverse]);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const key = orientation === 'vertical'
      ? event.key === 'ArrowRight' ? 8 : event.key === 'ArrowLeft' ? -8 : 0
      : event.key === 'ArrowDown' ? 8 : event.key === 'ArrowUp' ? -8 : 0;
    if (key === 0) return;
    event.preventDefault();
    onResize(reverse ? -key : key);
  };

  return (
    <div
      className={clsx('ide-pane-resizer', orientation === 'vertical' ? 'is-vertical' : 'is-horizontal')}
      role="separator"
      aria-orientation={orientation}
      aria-label={label}
      tabIndex={0}
      title={`Drag to resize ${label.toLowerCase()}`}
      onPointerDown={handlePointerDown}
      onKeyDown={handleKeyDown}
    />
  );
};
