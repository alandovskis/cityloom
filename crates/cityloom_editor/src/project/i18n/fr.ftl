# Ce que dit la liste des projets (src/project/view.rs, src/project/vm.rs, src/project/text.rs, web/index.html).
# Le titre de la fenêtre de la page d’accueil.
project-page-title = Projets CityLoom
project-list-heading = Vos projets
project-skip = Aller à vos projets
project-empty = Vous n’avez encore aucun projet. Nommez le premier ci-dessous pour commencer.
project-create-heading = Nouveau projet
project-create-name = Nom du projet
project-create-submit = Créer le projet
project-metro = Région métropolitaine
project-metro-fixed = La région métropolitaine ne peut plus être changée une fois le projet créé.
project-changed = Modifié le { $when }
# Un jour de l’année, comme l’écrit la liste des projets : 12 novembre 2026.
project-date = { $day ->
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
project-timeline = Chronologie
project-timeline-named = Chronologie de { $name }
# Le lien de la rangée du projet vers la page qui partage le projet par un lien (#221).
project-share = Partager
project-share-named = Partager { $name }
project-rename = Renommer
project-rename-named = Renommer { $name }
project-rename-save = Enregistrer le nom
project-cancel = Annuler
project-delete = Supprimer
project-delete-named = Supprimer { $name }
project-delete-confirm = Supprimer { $name } et tout ce qu’il contient? Cette action est irréversible.
project-delete-yes = Supprimer le projet
# Avant de supprimer un projet qui a des liens de partage créés sur cet appareil (#221) : ils sont d’abord supprimés sur le serveur.
project-delete-confirm-links = { $count ->
    [one] Supprimer { $name } et tout ce qu’il contient? Son lien de partage créé sur cet appareil cessera aussi de fonctionner. Cette action est irréversible.
   *[other] Supprimer { $name } et tout ce qu’il contient? Ses { $count } liens de partage créés sur cet appareil cesseront aussi de fonctionner. Cette action est irréversible.
}
# Ses liens de partage n’ont pas pu être supprimés sur le serveur, donc le projet a été conservé; $reason dit pourquoi, en une phrase.
project-delete-links-failed = { $name } n’a pas été supprimé : ses liens de partage n’ont pas pu être supprimés d’abord. { $reason }
# La raison pour laquelle le projet a été conservé ($failed, project-delete-links-failed), et ce qui peut encore être fait.
project-delete-anyway-offer = { $failed } Vous pouvez le supprimer quand même : ses liens de partage fonctionneront alors, et compteront dans la limite de liens actifs de votre compte, jusqu’à leur expiration.
project-delete-anyway = Supprimer quand même
# Pendant que les liens de partage d’un projet sont supprimés sur le serveur, avant le projet (#221); Annuler et Échap attendent.
project-delete-ending = Suppression de ses liens de partage sur le serveur… Le projet est supprimé une fois qu’ils le sont; d’ici là, cette action ne peut pas être annulée.
project-delete-waiting = Le serveur a demandé d’attendre un instant : ses liens de partage seront redemandés sous peu… Le projet est supprimé une fois qu’ils le sont; d’ici là, cette action ne peut pas être annulée.
project-gone = Ce projet n’est plus ici.
project-first-name = Premier projet
# A project whose places came before its record, until its record or a rename names it.
project-placeholder-name = Projet
project-refused-empty = Donnez un nom au projet.
project-refused-long = Le nom d’un projet compte au plus 80 caractères.
project-refused-unusable = La liste des projets n’a pas pu être modifiée sur cet appareil.
project-created = Projet { $name } créé.
project-renamed = Projet renommé { $name }.
project-deleted = Projet { $name } supprimé.
