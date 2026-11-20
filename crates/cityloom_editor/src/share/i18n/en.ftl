# What sharing a design says (src/share/view.rs, src/share/vm.rs, src/share/text.rs, src/share/model.rs, web/share.html).
# The share page's window title.
share-page-title = CityLoom share
share-skip = Skip to sharing
share-heading = Share { $project }
share-back = Back to the map
share-intro = A share link lets anyone who has it see this project as it is now, read-only and with no account: its streets, junctions and area rules. Changes you make later are not in it.
share-retention = The server keeps each shared copy for { $days } days, then deletes it, and its link then says it has expired. You can delete a link sooner here, and deleting your account deletes them all.
share-create = Create a share link
share-links-heading = Links made on this device
share-links-empty = No share link has been made on this device for this project yet.
share-link-label = Share link
share-copy = Copy link
share-delete = Delete link
# Until when a link works: a day, as share-date writes it.
share-until = Works until { $date }
share-expired-on = Expired on { $date }
# A day of the year: April 15, 2027.
share-date = { $month ->
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
share-created = Share link created. It works until { $date }.
share-copied = Link copied.
share-copy-failed = The link could not be copied. Select it and copy it yourself.
share-deleted = Share link deleted. Anyone who opens it is told it was deleted.
# The page that opens a share link.
share-open-heading = Shared design
share-opening = Opening the shared design…
share-open-projects = Go to your projects
# Why a share could not be opened, made or deleted.
share-error-bad-link = This is not a whole share link. Check that it was copied in full. Nothing was kept on this device.
share-error-not-found = No shared design has this link. It may have been copied wrong, or deleted with the account that shared it. Nothing was kept on this device.
share-error-expired = This shared design has expired: a share link works for { $days } days. Ask the person who shared it for a new link. Nothing was kept on this device.
share-error-deleted = The person who shared this design deleted it. Nothing was kept on this device.
share-error-newer = This shared design was made with a newer CityLoom than this page. Reload the page to get the newest version. Nothing was kept on this device.
share-error-damaged = This shared design is damaged and cannot be opened. Nothing was kept on this device.
share-error-network = The server could not be reached. Check your connection and try again.
share-error-unavailable = The server that keeps shared designs cannot answer now. Try again later.
share-error-storage = This browser could not keep a copy of the shared design: its storage is full or blocked. Nothing was kept on this device.
share-error-signed-out = Sign in to share this project: the server keeps shared copies for your account.
share-error-read-only = This design is open read-only: it cannot be shared from here.
share-error-unusable = This project's work cannot be read, so it cannot be shared.
share-error-too-large = This project is too large to share: a share holds at most { $megabytes } MB.
share-error-full = You have { $count } share links already. Delete one before making another.
share-error-not-yours = Another account made this link: only the account that made it can delete it.
share-error-refused = The server refused the request.
