# Ce que dit la liste des projets (src/project/view.rs, src/project/vm.rs, src/project/text.rs, web/index.html).
project-list-heading = Vos projets
project-skip = Aller à vos projets
project-empty = Vous n’avez encore aucun projet. Nommez le premier ci-dessous pour commencer.
project-create-heading = Nouveau projet
project-create-name = Nom du projet
project-create-submit = Créer le projet
project-area = Lieu
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
project-rename = Renommer
project-rename-named = Renommer { $name }
project-rename-save = Enregistrer le nom
project-cancel = Annuler
project-delete = Supprimer
project-delete-named = Supprimer { $name }
project-delete-confirm = Supprimer { $name } et tout ce qu’il contient? Cette action est irréversible.
project-delete-yes = Supprimer le projet
project-gone = Ce projet n’est plus ici.
project-first-name = Premier projet
project-refused-empty = Donnez un nom au projet.
project-refused-long = Le nom d’un projet compte au plus 80 caractères.
project-refused-unusable = La liste des projets n’a pas pu être modifiée sur cet appareil.
project-created = Projet { $name } créé.
project-renamed = Projet renommé { $name }.
project-deleted = Projet { $name } supprimé.
project-about-heading = À propos de CityLoom
