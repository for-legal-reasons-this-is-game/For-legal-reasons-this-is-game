import type { Properties, Property } from './types';
/**
 * - `event`: include the stored key/value on captured events as-is.
 * - `hidden`: keep the key in persistence only; never expose it on captured events.
 * - `derived`: do not expose the stored key directly, but derive one or more event properties from its value.
 *   For example, `ENABLED_FEATURE_FLAGS` is stored as `$enabled_feature_flags`, but exposed on events as
 *   `$feature/<flag-key>` properties via `transformToEventProperties`.
 */
export type PersistenceKeyExposure = 'event' | 'hidden' | 'derived';
/**
 * Keys sharing a `storageGroup` are persisted together in their own storage
 * entry (`<name>__<group>`) instead of the main persistence blob, when the
 * `split_storage` config is enabled. Keys without a group
 * stay in the main blob. Members of one group are written in a single entry so
 * an atomic `register` of the whole group lands as one write (no torn state).
 */
export type PersistenceStorageGroup = 'flags' | 'surveys';
export declare const PERSISTENCE_STORAGE_GROUPS: readonly PersistenceStorageGroup[];
interface PersistenceKeyPolicyEntry {
    exposure: PersistenceKeyExposure;
    storageGroup?: PersistenceStorageGroup;
    /**
     * Per-request metadata that changes on every remote load even when the
     * meaningful content is unchanged. A volatile key never marks its storage
     * group dirty and is excluded from the group fingerprint, so a
     * volatile-only change does not rewrite — and cross-tab re-broadcast — a
     * large group entry. The freshest value rides along on the next content
     * write; the on-disk copy may lag in-memory until then.
     */
    volatile?: boolean;
    shouldSkipFromEventProperties?: (value: Property, shouldSkip: () => boolean) => boolean;
    transformToEventProperties?: (value: Property) => Properties;
}
export declare const PERSISTENCE_KEY_POLICY: Record<string, PersistenceKeyPolicyEntry>;
export declare const getPersistenceKeyPolicy: (key: string) => PersistenceKeyPolicyEntry | undefined;
export {};
