import { toUnits } from "@/lib/decimal";
import type { Position } from "@/types/Position";

// What an account can spend, in its asset's smallest units: the posted balance minus pending
// debits. An order's hold is a posted transfer to the order's own hold account, so it has
// already left the posted balance; pending debits only appear while an account is closing.
export function availableUnits(position: Position): bigint | null {
    const posted = toUnits(position.net_posted, position.decimals);
    const pending = toUnits(position.debits_pending, position.decimals);
    return posted !== null && pending !== null ? posted - pending : null;
}
