-- A design milestone is a record too: kind 'milestone', its key `<milestone id>` (bare: the first project's) or `<project>:<milestone id>`. Existing rows are untouched.
alter table work drop constraint work_kind_check;
alter table work add constraint work_kind_check check (kind in ('street', 'junction', 'rule', 'area', 'project', 'milestone'));
