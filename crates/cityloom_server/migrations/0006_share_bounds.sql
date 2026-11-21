-- The checks made before a share is kept (`db/share.rs`), each over an index that the markers (rows whose body is
-- gone, kept a year past their expiry) do not grow: the live shares an account holds (`MAX_LIVE_SHARES`) are counted
-- over the rows that still have a body, and the shares it made in the last day (`MAX_SHARES_PER_DAY`, deleted ones
-- too) over the account's rows by when they were made. That second index also finds an account's rows when it is
-- deleted, as `shares_user` did.
drop index shares_user;
create index shares_user_created on shares (user_id, created_at);
create index shares_live on shares (user_id, expires_at) where body is not null;
