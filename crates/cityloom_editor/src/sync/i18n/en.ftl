# What syncing says (src/sync/text.rs, src/sync/vm.rs; the editors say sync-updated and sync-replaced by key).
sync-status-synced = Synced
sync-status-pending = Not synced yet
sync-status-paused = Syncing is paused
sync-updated = Updated from another device.
sync-replaced = Your change to this place was replaced by a later one.
sync-paused = Syncing is paused: some of the work saved in this browser was written by a newer version of CityLoom or cannot be read, so it is neither sent nor changed.
sync-merged = Your work was merged with your account: { $sent ->
    [one] 1 place sent
   *[other] { $sent } places sent
  }, { $received ->
    [one] 1 place received.
   *[other] { $received } places received.
  }
sync-too-large-record = A place is too big for the server (64 KB), so it stays on this device only.
sync-too-large-records = The server keeps at most 5,000 records per account, so the rest stays on this device only.
sync-too-large-bytes = The server keeps at most 2 MB of work per account, so the rest stays on this device only.
sync-too-large-other = Some of your work is too big for the server, so it stays on this device only.
