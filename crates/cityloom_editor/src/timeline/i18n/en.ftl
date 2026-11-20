# What the design timeline says (src/timeline/view.rs, src/timeline/vm.rs, src/timeline/text.rs, web/timeline.html).
# The timeline page's window title.
timeline-page-title = CityLoom timeline
timeline-skip = Skip to the timeline
timeline-heading = Timeline of { $project }
timeline-back = Back to the map
timeline-create-heading = Save a milestone
timeline-create-name = Milestone name
timeline-create-submit = Save milestone
timeline-create-hint = A milestone keeps the streets, junctions and rules of this project as they are now.
timeline-empty = No milestones yet. Save one to be able to come back to this design.
timeline-list-heading = Milestones, oldest first
timeline-auto = Saved before a restore
# When a milestone was made: a date and a time, in UTC.
timeline-when = { $date }, { $time } UTC
# A day of the year, as the timeline writes it: November 12, 2026.
timeline-date = { $month ->
    [1] January
    [2] February
    [3] March
    [4] April
    [5] May
    [6] June
    [7] July
    [8] August
    [9] September
    [10] October
    [11] November
   *[12] December
} { $day }, { $year }
timeline-by = By { $author }
timeline-version = CityLoom { $version }
timeline-summary = { $streets ->
    [one] 1 street
   *[other] { $streets } streets
}, { $junctions ->
    [one] 1 junction
   *[other] { $junctions } junctions
}, { $rules ->
    [one] 1 rule
   *[other] { $rules } rules
}
timeline-summary-empty = No streets, junctions or rules edited
timeline-compare = Compare with now
timeline-compare-named = Compare with now: { $name }
timeline-compare-close = Close the comparison
timeline-compare-heading = Since { $name }
timeline-compare-none = The work is the same as in { $name }.
timeline-compare-count = { $n ->
    [one] 1 place differs from { $name }.
   *[other] { $n } places differ from { $name }.
}
timeline-compare-unusable = The work on this device cannot be read, so it cannot be compared.
timeline-how-added = Added since
timeline-how-removed = Removed since
timeline-how-changed = Changed
timeline-street = { $name }
timeline-street-unnamed = Unnamed street
timeline-junction = Junction at node { $node }
timeline-junction-unknown = Junction
timeline-rule = Rule { $name }
timeline-rule-unnamed = Area rule
timeline-aspect-name = name
timeline-aspect-class = street class
timeline-aspect-width = width
timeline-aspect-lanes = lanes
timeline-aspect-speed = speed limit
timeline-aspect-zones = changes along the street
timeline-aspect-arms = arms
timeline-aspect-control = control
timeline-aspect-rule = rule
timeline-aspect-outline = outline
timeline-aspect-other = other details
timeline-restore = Restore
timeline-restore-named = Restore { $name }
timeline-restore-confirm = Put the work back as it was in { $name }? The work as it is now is saved first as a milestone, so you can come back to it.
timeline-restore-yes = Restore this milestone
timeline-cancel = Cancel
timeline-delete = Delete
timeline-delete-named = Delete { $name }
timeline-delete-confirm = Delete the milestone { $name }? The work itself is not changed.
timeline-delete-yes = Delete the milestone
# The name of the milestone saved before a restore.
timeline-backup-name = Before restoring { $name }
timeline-created = Milestone { $name } saved.
timeline-restored = Restored { $name }. The earlier work is saved as { $backup }.
timeline-restored-same = The work already is as it was in { $name }.
timeline-deleted = Milestone { $name } deleted.
timeline-gone = That milestone is no longer here.
timeline-refused-empty = Give the milestone a name.
timeline-refused-long = A milestone name has at most 80 characters.
timeline-refused-large = The work is too large to keep as one milestone.
timeline-refused-full = The milestones of this project take all the room they may on this device. Delete one to make room.
timeline-refused-unusable = The milestones or the work on this device could not be read or changed.
timeline-refused-stale = This milestone was saved before the map was rebuilt with one street per block, so it cannot be restored. You can still compare it with now.
