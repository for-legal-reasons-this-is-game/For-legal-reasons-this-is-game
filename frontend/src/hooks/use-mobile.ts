import * as React from "react"

const MOBILE_BREAKPOINT = 768
const MOBILE_QUERY = `(max-width: ${MOBILE_BREAKPOINT - 1}px)`

function subscribe(onChange: () => void) {
  const mql = window.matchMedia(MOBILE_QUERY)
  mql.addEventListener("change", onChange)
  return () => mql.removeEventListener("change", onChange)
}

// Changed from shadcn's version, which set state inside an effect (flagged by
// react-hooks/set-state-in-effect) and returned false on the first render.
// New React: useSyncExternalStore reads a value that lives outside React (here the
// media query) during render, and re-renders when `subscribe` reports a change.
export function useIsMobile() {
  return React.useSyncExternalStore(
    subscribe,
    () => window.matchMedia(MOBILE_QUERY).matches,
    () => false, // value used when rendering on a server (not used in this app)
  )
}
