import { window as external_globals_mjs_window } from "./globals.mjs";
const isAngularZonePresent = ()=>!!external_globals_mjs_window?.Zone;
const isDocument = (x)=>'undefined' != typeof Document && x instanceof Document;
export { isAngularZonePresent, isDocument };
