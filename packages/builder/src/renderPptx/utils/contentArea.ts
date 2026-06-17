type Padding =
  | number
  | {
      top?: number;
      right?: number;
      bottom?: number;
      left?: number;
    };

interface ContentArea {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Compute the content rendering area, accounting for the node's padding.
 * Draw background/border across the node's full region (node.x/y/w/h),
 * Draw content into the region returned by this function.
 */
export function getContentArea(node: {
  x: number;
  y: number;
  w: number;
  h: number;
  padding?: Padding;
}): ContentArea {
  if (node.padding === undefined) {
    return { x: node.x, y: node.y, w: node.w, h: node.h };
  }

  let top: number, right: number, bottom: number, left: number;

  if (typeof node.padding === "number") {
    top = right = bottom = left = node.padding;
  } else {
    top = node.padding.top ?? 0;
    right = node.padding.right ?? 0;
    bottom = node.padding.bottom ?? 0;
    left = node.padding.left ?? 0;
  }

  return {
    x: node.x + left,
    y: node.y + top,
    w: Math.max(0, node.w - left - right),
    h: Math.max(0, node.h - top - bottom),
  };
}
