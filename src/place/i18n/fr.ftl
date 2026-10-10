# Ce que dit la recherche de lieux de la page d’accueil (src/place/vm.rs, src/place/view.rs), et pourquoi les rues d’un lieu n’ont pas pu être obtenues (src/place/loader.rs).

## La boîte de recherche
place-search-label = Chercher un lieu
place-search-placeholder = Chercher un lieu, comme Kreuzberg, Berlin
place-results = Lieux trouvés
place-find = Chercher
place-open = Ouvrir

## Où en est la recherche
place-searching = Recherche en cours…
place-found = { $n ->
    [one] { $n } lieu trouvé
   *[other] { $n } lieux trouvés
}. Utilisez les flèches, puis Entrée.
place-nothing = Aucun lieu trouvé. Essayez une ville, un quartier ou une adresse.
# `name` est le nom propre du lieu, tel que le donne la recherche.
place-loading = Chargement des rues de { $name }…

## Pourquoi les rues d’un lieu n’ont pas pu être obtenues
# `e` est l’erreur d’origine, telle que la formule le navigateur ou une bibliothèque (non traduite). `area` est le nom du lieu.
place-fetch-failed = les rues de { $area } n’ont pas pu être obtenues ({ $e })
place-no-streets = OpenStreetMap n’a aucune rue autour de { $area }
place-roads-not-kept = les rues n’ont pas pu être conservées : le stockage est bloqué ou plein
place-roads-not-understood = les rues n’ont pas été comprises ({ $e })
place-search-unexpected = la recherche de lieux a répondu quelque chose d’inattendu ({ $e })
place-index-not-understood = l’index des tuiles n’a pas été compris ({ $e })
place-index-no-size = l’index des tuiles n’a pas de taille de tuile
place-area-not-kept = Le lieu n’a pas pu être conservé : le stockage est bloqué.
# Ce qui n’a pas fonctionné, tel que l’a dit le navigateur ou le lecteur OpenStreetMap, transmis tel quel.
place-problem = { $e }
