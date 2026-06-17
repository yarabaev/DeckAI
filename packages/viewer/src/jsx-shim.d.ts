// React 19 removed the global `JSX` namespace declaration that React
// 18 shipped. Re-export `React.JSX` into the global namespace so this
// package's existing `JSX.Element` return-type annotations keep
// resolving without rewriting every component.
//
// Remove this file once every annotation has been migrated to either
// `React.JSX.Element` or `import { type JSX } from "react"`.
import type { JSX as ReactJSX } from "react";

declare global {
  namespace JSX {
    type Element = ReactJSX.Element;
    type ElementClass = ReactJSX.ElementClass;
    type ElementAttributesProperty = ReactJSX.ElementAttributesProperty;
    type ElementChildrenAttribute = ReactJSX.ElementChildrenAttribute;
    type LibraryManagedAttributes<C, P> = ReactJSX.LibraryManagedAttributes<
      C,
      P
    >;
    type IntrinsicAttributes = ReactJSX.IntrinsicAttributes;
    type IntrinsicClassAttributes<T> = ReactJSX.IntrinsicClassAttributes<T>;
    type IntrinsicElements = ReactJSX.IntrinsicElements;
  }
}
