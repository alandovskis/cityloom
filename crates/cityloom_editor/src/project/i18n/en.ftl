# What the list of projects says (src/project/view.rs, src/project/vm.rs, src/project/text.rs, web/index.html).
project-list-heading = Your projects
project-skip = Skip to your projects
project-empty = You have no projects yet. Name your first one below to start.
project-create-heading = New project
project-create-name = Project name
project-create-submit = Create project
project-area = Area
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
project-rename = Rename
project-rename-named = Rename { $name }
project-rename-save = Save the name
project-cancel = Cancel
project-delete = Delete
project-delete-named = Delete { $name }
project-delete-confirm = Delete { $name } and everything in it? This cannot be undone.
project-delete-yes = Delete the project
project-gone = That project is no longer here.
project-first-name = First project
project-refused-empty = Give the project a name.
project-refused-long = A project name has at most 80 characters.
project-refused-unusable = The list of projects could not be changed on this device.
project-created = Project { $name } created.
project-renamed = Project renamed to { $name }.
project-deleted = Project { $name } deleted.
project-about-heading = About CityLoom
