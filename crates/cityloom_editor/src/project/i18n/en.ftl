# What the list of projects says (src/project/view.rs, src/project/vm.rs, src/project/text.rs, web/index.html).
# The home page's window title.
project-page-title = CityLoom projects
project-list-heading = Your projects
project-skip = Skip to your projects
project-empty = You have no projects yet. Name your first one below to start.
project-create-heading = New project
project-create-name = Project name
project-create-submit = Create project
project-metro = Metropolitan area
project-metro-fixed = The metropolitan area cannot be changed once the project is created.
project-changed = Changed { $when }
# A day of the year, as the list of projects writes it: November 12, 2026.
project-date = { $month ->
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
project-timeline = Timeline
project-timeline-named = Timeline of { $name }
# The project row's link to the page that shares the project by a link (#221).
project-share = Share
project-share-named = Share { $name }
project-rename = Rename
project-rename-named = Rename { $name }
project-rename-save = Save the name
project-cancel = Cancel
project-delete = Delete
project-delete-named = Delete { $name }
project-delete-confirm = Delete { $name } and everything in it? This cannot be undone.
project-delete-yes = Delete the project
# Before deleting a project that has share links made on this device (#221): they are deleted on the server first.
project-delete-confirm-links = { $count ->
    [one] Delete { $name } and everything in it? Its share link made on this device stops working too. This cannot be undone.
   *[other] Delete { $name } and everything in it? Its { $count } share links made on this device stop working too. This cannot be undone.
}
# Its share links could not be deleted on the server, so the project was kept; $reason says why, as a sentence.
project-delete-links-failed = { $name } was not deleted: its share links could not be deleted first. { $reason }
# The reason the project was kept ($failed, project-delete-links-failed), and what can still be done.
project-delete-anyway-offer = { $failed } You can delete it anyway: its share links then keep working until they expire.
project-delete-anyway = Delete anyway
# While a project's share links are deleted on the server, before the project (#221); Cancel and Escape wait meanwhile.
project-delete-ending = Deleting its share links on the server… The project is deleted once they are; until then this cannot be cancelled.
project-gone = That project is no longer here.
project-first-name = First project
# A project whose places came before its record, until its record or a rename names it.
project-placeholder-name = Project
project-refused-empty = Give the project a name.
project-refused-long = A project name has at most 80 characters.
project-refused-unusable = The list of projects could not be changed on this device.
project-created = Project { $name } created.
project-renamed = Project renamed to { $name }.
project-deleted = Project { $name } deleted.
