-- A project is a record too: kind 'project', its key the project id. Existing rows are untouched (bare keys belong to the first project).
-- 0002 declared the check inline on the column, so Postgres named it work_kind_check.
alter table work drop constraint work_kind_check;
alter table work add constraint work_kind_check check (kind in ('street', 'junction', 'rule', 'area', 'project'));
