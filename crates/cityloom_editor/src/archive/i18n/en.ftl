# What the design archive section of the Account page says (src/archive/).
archive-heading = Design archive
archive-lead = Save your street, junction and area-rule work as a file, for a backup or to hand to someone else, or add the work from such a file to this device. The file does not hold your account, your language or theme, or the downloaded street networks.
archive-export = Export design
archive-import = Import design…
archive-import-help = Importing adds what is not on this device yet. A place that is here already and differs is never replaced unless you choose to.
archive-part-header = Header
archive-part-edits = Street and junction edits
archive-part-rules = Area rules
archive-exported = Saved { $name } with { $count ->
    [one] one record of your design work.
   *[other] { $count } records of your design work.
}
archive-export-empty = You have no design work to export yet, so no file was made.
archive-export-failed = The file could not be offered for download. Your work on this device was not changed.
archive-imported = Import finished. Added: { $added }. Replaced: { $replaced }. Kept as they were here: { $kept }. Already identical: { $same }.
archive-nothing-new = Nothing to import: everything in this archive is already on this device.
archive-cancelled = Import cancelled. Nothing on this device was changed.
archive-confirm-heading = Some places differ from this device
archive-confirm-summary = In this archive, { $differ } differ from the same places on this device, and { $new } are new here and will be added either way. Nothing has been changed yet.
archive-confirm-detail = Streets: { $streets }. Junctions: { $junctions }. Area rules: { $rules }.
archive-confirm-keep = Keep mine
archive-confirm-use = Use the archive's
archive-confirm-cancel = Cancel
archive-error-too-large = This file is larger than { $megabytes } MB, more than a CityLoom design archive can be. Nothing on this device was changed.
archive-error-not-json = This file is not a CityLoom design archive: it could not be read as JSON, and may be cut short or damaged. Nothing on this device was changed.
archive-error-not-archive = This file is JSON, but it is not a CityLoom design archive. Choose a file saved with Export design. Nothing on this device was changed.
archive-error-damaged = This archive is damaged: its { $part } cannot be read. Nothing on this device was changed.
archive-error-future-format = This archive uses format version { $version }, made by a newer CityLoom than this one. Update CityLoom and try again. Nothing on this device was changed.
archive-error-edits-newer = The street and junction edits in this archive are version { $version }, saved by a newer CityLoom than this one. Update CityLoom and try again. Nothing on this device was changed.
archive-error-edits-old = The street and junction edits in this archive are version { $version }, older than this CityLoom can read. Nothing on this device was changed.
archive-error-rules-newer = The area rules in this archive are version { $version }, saved by a newer CityLoom than this one. Update CityLoom and try again. Nothing on this device was changed.
archive-error-local-edits-newer = The street and junction edits on this device were saved by a newer CityLoom, so this one does not change them. Nothing was imported and nothing on this device was changed.
archive-error-local-edits-damaged = The street and junction edits on this device cannot be read, so nothing was imported, to protect them. Nothing on this device was changed.
archive-error-local-rules-newer = The area rules on this device were saved by a newer CityLoom, so this one does not change them. Nothing was imported and nothing on this device was changed.
archive-error-local-rules-damaged = The area rules on this device cannot be read, so nothing was imported, to protect them. Nothing on this device was changed.
archive-error-storage = This browser would not keep the imported work; its storage may be full or blocked. What had been written was put back, so nothing on this device was changed.
archive-error-export = Your street and junction edits or area rules on this device cannot be read, so they were not exported. Nothing on this device was changed.
