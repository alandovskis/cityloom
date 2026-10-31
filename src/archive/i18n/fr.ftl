# Ce que dit la section d’archive de conception de la page Compte (src/archive/).
archive-heading = Archive de conception
archive-lead = Enregistrez votre travail sur les rues, les intersections et les règles de zone dans un fichier, pour une sauvegarde ou pour le confier à quelqu’un, ou ajoutez à cet appareil le travail d’un tel fichier. Le fichier ne contient ni votre compte, ni votre langue ou votre thème, ni les réseaux de rues téléchargés.
archive-export = Exporter la conception
archive-import = Importer une conception…
archive-import-help = L’importation ajoute ce qui n’est pas encore sur cet appareil. Un lieu déjà présent et différent n’est jamais remplacé, sauf si vous le choisissez.
archive-part-header = En-tête
archive-part-edits = Modifications des rues et des intersections
archive-part-rules = Règles de zone
archive-exported = { $name } enregistré avec { $count ->
    [one] un enregistrement de votre travail de conception.
   *[other] { $count } enregistrements de votre travail de conception.
}
archive-export-empty = Vous n’avez pas encore de travail de conception à exporter; aucun fichier n’a été créé.
archive-export-failed = Le fichier n’a pas pu être proposé au téléchargement. Votre travail sur cet appareil n’a pas été modifié.
archive-imported = Importation terminée. Ajouts : { $added }. Remplacements : { $replaced }. Conservés tels quels ici : { $kept }. Déjà identiques : { $same }.
archive-nothing-new = Rien à importer : tout le contenu de cette archive est déjà sur cet appareil.
archive-cancelled = Importation annulée. Rien n’a été modifié sur cet appareil.
archive-confirm-heading = Certains lieux diffèrent de ceux de cet appareil
archive-confirm-summary = Dans cette archive, { $differ } diffèrent des mêmes lieux sur cet appareil, et { $new } sont nouveaux ici et seront ajoutés dans les deux cas. Rien n’a encore été modifié.
archive-confirm-detail = Rues : { $streets }. Intersections : { $junctions }. Règles de zone : { $rules }.
archive-confirm-keep = Garder les miens
archive-confirm-use = Utiliser ceux de l’archive
archive-confirm-cancel = Annuler
archive-error-too-large = Ce fichier dépasse { $megabytes } Mo, plus qu’une archive de conception CityLoom ne peut contenir. Rien n’a été modifié sur cet appareil.
archive-error-not-json = Ce fichier n’est pas une archive de conception CityLoom : il n’a pas pu être lu comme du JSON, et il est peut-être tronqué ou endommagé. Rien n’a été modifié sur cet appareil.
archive-error-not-archive = Ce fichier est du JSON, mais ce n’est pas une archive de conception CityLoom. Choisissez un fichier enregistré avec Exporter la conception. Rien n’a été modifié sur cet appareil.
archive-error-damaged = Cette archive est endommagée : son contenu « { $part } » est illisible. Rien n’a été modifié sur cet appareil.
archive-error-future-format = Cette archive utilise la version { $version } du format, créée par une version de CityLoom plus récente que celle-ci. Mettez CityLoom à jour et réessayez. Rien n’a été modifié sur cet appareil.
archive-error-edits-newer = Les modifications des rues et des intersections de cette archive sont en version { $version }, enregistrées par une version de CityLoom plus récente que celle-ci. Mettez CityLoom à jour et réessayez. Rien n’a été modifié sur cet appareil.
archive-error-edits-old = Les modifications des rues et des intersections de cette archive sont en version { $version }, plus ancienne que ce que cette version de CityLoom peut lire. Rien n’a été modifié sur cet appareil.
archive-error-rules-newer = Les règles de zone de cette archive sont en version { $version }, enregistrées par une version de CityLoom plus récente que celle-ci. Mettez CityLoom à jour et réessayez. Rien n’a été modifié sur cet appareil.
archive-error-local-edits-newer = Les modifications des rues et des intersections de cet appareil ont été enregistrées par une version de CityLoom plus récente, et celle-ci ne les modifie donc pas. Rien n’a été importé et rien n’a été modifié sur cet appareil.
archive-error-local-edits-damaged = Les modifications des rues et des intersections de cet appareil sont illisibles; rien n’a donc été importé, afin de les protéger. Rien n’a été modifié sur cet appareil.
archive-error-local-rules-newer = Les règles de zone de cet appareil ont été enregistrées par une version de CityLoom plus récente, et celle-ci ne les modifie donc pas. Rien n’a été importé et rien n’a été modifié sur cet appareil.
archive-error-local-rules-damaged = Les règles de zone de cet appareil sont illisibles; rien n’a donc été importé, afin de les protéger. Rien n’a été modifié sur cet appareil.
archive-error-storage = Ce navigateur n’a pas conservé le travail importé; son espace de stockage est peut-être plein ou bloqué. Ce qui avait été écrit a été remis en place : rien n’a été modifié sur cet appareil.
archive-error-export = Vos modifications des rues et des intersections ou vos règles de zone sur cet appareil sont illisibles; elles n’ont donc pas été exportées. Rien n’a été modifié sur cet appareil.
