# Ce que dit le partage d’un aménagement (src/share/view.rs, src/share/vm.rs, src/share/text.rs, src/share/model.rs, web/share.html).
# Le titre de la fenêtre de la page de partage.
share-page-title = Partage CityLoom
share-skip = Aller au partage
share-heading = Partager { $project }
share-back = Retour à la carte
share-intro = Un lien de partage permet à quiconque l’a de consulter ce projet tel qu’il est maintenant, en lecture seule et sans compte : ses rues, ses jonctions et ses règles de zone. Les modifications que vous ferez ensuite n’y seront pas.
share-retention = Le serveur conserve chaque copie partagée pendant { $days } jours, puis la supprime; son lien indique alors qu’il a expiré. Vous pouvez supprimer un lien plus tôt ici, et supprimer votre compte les supprime tous.
share-create = Créer un lien de partage
share-links-heading = Liens créés sur cet appareil
share-links-empty = Aucun lien de partage n’a encore été créé sur cet appareil pour ce projet.
share-link-label = Lien de partage
share-copy = Copier le lien
share-delete = Supprimer le lien
# Jusqu’à quand un lien fonctionne : un jour, comme l’écrit share-date.
share-until = Fonctionne jusqu’au { $date }
share-expired-on = A expiré le { $date }
# Un jour de l’année : 15 avril 2027.
share-date = { $day ->
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
share-created = Lien de partage créé. Il fonctionne jusqu’au { $date }.
share-copied = Lien copié.
share-copy-failed = Le lien n’a pas pu être copié. Sélectionnez-le et copiez-le vous-même.
share-deleted = Lien de partage supprimé. Quiconque l’ouvre apprend qu’il a été supprimé.
# La page qui ouvre un lien de partage.
share-open-heading = Aménagement partagé
share-opening = Ouverture de l’aménagement partagé…
share-open-projects = Aller à vos projets
# Pourquoi un lien n’a pas pu être ouvert, là où créer ou supprimer un lien le dit autrement : rien n’a été conservé.
share-open-error-network = Le serveur n’a pas pu être joint. Vérifiez votre connexion et réessayez. Rien n’a été conservé sur cet appareil.
share-open-error-unavailable = Le serveur qui conserve les aménagements partagés ne peut pas répondre pour le moment. Réessayez plus tard. Rien n’a été conservé sur cet appareil.
share-open-error-refused = Le serveur a refusé d’ouvrir ce lien. Rien n’a été conservé sur cet appareil.
# Pourquoi un partage n’a pas pu être ouvert, créé ou supprimé.
share-error-bad-link = Ce n’est pas un lien de partage complet. Vérifiez qu’il a été copié en entier. Rien n’a été conservé sur cet appareil.
share-error-not-found = Aucun aménagement partagé n’a ce lien. Il a peut-être été mal copié, ou supprimé avec le compte qui l’a partagé. Rien n’a été conservé sur cet appareil.
share-error-expired = Cet aménagement partagé a expiré : un lien de partage fonctionne pendant { $days } jours. Demandez un nouveau lien à la personne qui l’a partagé. Rien n’a été conservé sur cet appareil.
share-error-deleted = La personne qui a partagé cet aménagement l’a supprimé. Rien n’a été conservé sur cet appareil.
share-error-newer = Cet aménagement partagé a été créé avec une version de CityLoom plus récente que cette page. Rechargez la page pour obtenir la dernière version. Rien n’a été conservé sur cet appareil.
share-error-damaged = Cet aménagement partagé est endommagé et ne peut pas être ouvert. Rien n’a été conservé sur cet appareil.
share-error-network = Le serveur n’a pas pu être joint. Vérifiez votre connexion et réessayez.
share-error-unavailable = Le serveur qui conserve les aménagements partagés ne peut pas répondre pour le moment. Réessayez plus tard.
share-error-storage = Ce navigateur n’a pas pu conserver une copie de l’aménagement partagé : son espace de stockage est plein ou bloqué. Rien n’a été conservé sur cet appareil.
share-error-signed-out = Connectez-vous pour partager ce projet : le serveur conserve les copies partagées pour votre compte.
share-error-read-only = Cet aménagement est ouvert en lecture seule : il ne peut pas être partagé d’ici.
share-error-unusable = Le travail de ce projet est illisible; il ne peut donc pas être partagé.
share-error-too-large = Ce projet est trop volumineux pour être partagé : un partage contient au plus { $megabytes } Mo.
share-error-full = Vous avez déjà { $count } liens de partage. Supprimez-en un avant d’en créer un autre.
share-error-not-yours = Un autre compte a créé ce lien : seul le compte qui l’a créé peut le supprimer.
share-error-refused = Le serveur a refusé la demande.
