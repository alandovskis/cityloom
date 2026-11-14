# Ce que dit la chronologie de conception (src/timeline/view.rs, src/timeline/vm.rs, src/timeline/text.rs, web/timeline.html).
# Le titre de la fenêtre de la page de la chronologie.
timeline-page-title = Chronologie CityLoom
timeline-skip = Aller à la chronologie
timeline-heading = Chronologie de { $project }
timeline-back = Retour à la carte
timeline-create-heading = Enregistrer un jalon
timeline-create-name = Nom du jalon
timeline-create-submit = Enregistrer le jalon
timeline-create-hint = Un jalon conserve les rues, les jonctions et les règles de ce projet tels qu’ils sont maintenant.
timeline-empty = Aucun jalon pour le moment. Enregistrez-en un pour pouvoir revenir à cette conception.
timeline-list-heading = Jalons, du plus ancien au plus récent
timeline-auto = Enregistré avant une restauration
# Quand un jalon a été créé : une date et une heure, en UTC.
timeline-when = { $date }, { $time } UTC
# Un jour de l’année, comme l’écrit la chronologie : 12 novembre 2026.
timeline-date = { $day ->
    [1] 1er
   *[other] { $day }
} { $month ->
    [1] janvier
    [2] février
    [3] mars
    [4] avril
    [5] mai
    [6] juin
    [7] juillet
    [8] août
    [9] septembre
    [10] octobre
    [11] novembre
   *[12] décembre
} { $year }
timeline-by = Par { $author }
timeline-version = CityLoom { $version }
timeline-summary = { $streets ->
    [one] { $streets } rue
   *[other] { $streets } rues
}, { $junctions ->
    [one] { $junctions } jonction
   *[other] { $junctions } jonctions
}, { $rules ->
    [one] { $rules } règle
   *[other] { $rules } règles
}
timeline-summary-empty = Aucune rue, jonction ni règle modifiée
timeline-compare = Comparer avec maintenant
timeline-compare-named = Comparer { $name } avec maintenant
timeline-compare-close = Fermer la comparaison
timeline-compare-heading = Depuis { $name }
timeline-compare-none = Le travail est identique à celui de { $name }.
timeline-compare-count = { $n ->
    [one] { $n } lieu diffère de { $name }.
   *[other] { $n } lieux diffèrent de { $name }.
}
timeline-compare-unusable = Le travail sur cet appareil ne peut pas être lu; il ne peut donc pas être comparé.
timeline-how-added = Ajouté depuis
timeline-how-removed = Retiré depuis
timeline-how-changed = Modifié
timeline-street = { $name }
timeline-street-unnamed = Rue sans nom
timeline-junction = Jonction au nœud { $node }
timeline-junction-unknown = Jonction
timeline-rule = Règle { $name }
timeline-rule-unnamed = Règle de zone
timeline-aspect-name = nom
timeline-aspect-class = classe de rue
timeline-aspect-width = largeur
timeline-aspect-lanes = voies
timeline-aspect-speed = limite de vitesse
timeline-aspect-zones = zones
timeline-aspect-arms = branches
timeline-aspect-control = régulation
timeline-aspect-rule = règle
timeline-aspect-outline = contour
timeline-aspect-other = autres détails
timeline-restore = Restaurer
timeline-restore-named = Restaurer { $name }
timeline-restore-confirm = Remettre le travail comme il était dans { $name }? Le travail actuel est d’abord enregistré comme jalon, pour que vous puissiez y revenir.
timeline-restore-yes = Restaurer ce jalon
timeline-cancel = Annuler
timeline-delete = Supprimer
timeline-delete-named = Supprimer { $name }
timeline-delete-confirm = Supprimer le jalon { $name }? Le travail lui-même n’est pas modifié.
timeline-delete-yes = Supprimer le jalon
# Le nom du jalon enregistré avant une restauration.
timeline-backup-name = Avant la restauration de { $name }
timeline-created = Jalon { $name } enregistré.
timeline-restored = { $name } restauré. Le travail précédent est enregistré sous { $backup }.
timeline-restored-same = Le travail est déjà comme il était dans { $name }.
timeline-deleted = Jalon { $name } supprimé.
timeline-gone = Ce jalon n’est plus ici.
timeline-refused-empty = Donnez un nom au jalon.
timeline-refused-long = Le nom d’un jalon compte au plus 80 caractères.
timeline-refused-large = Le travail est trop volumineux pour être conservé dans un seul jalon.
timeline-refused-unusable = Les jalons ou le travail sur cet appareil n’ont pas pu être lus ou modifiés.
