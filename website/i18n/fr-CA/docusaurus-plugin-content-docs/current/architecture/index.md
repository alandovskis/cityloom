---
title: Architecture
slug: /architecture
---

# Architecture

Cette page est la vue d’ensemble de l’assemblage du code de CityLoom. La section Architecture de la barre latérale
contiendra d’autres pages à côté d’elle. Les chemins des fichiers de l’éditeur sont relatifs à
`crates/cityloom_editor/src/`.

## Tranches verticales

Le code est organisé par fonctionnalité, un dossier par tranche, et non par couche. Le nouveau code va dans la tranche
qui possède la fonctionnalité.

| Tranche        | Contenu                                                                                                                                                                                                                                                                                                                |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `account/`     | La page Compte : connexions associées, lieu de conservation du travail, suppression du compte.                                                                                                                                                                                                                         |
| `archive/`     | L’archive de conception : exportation et importation des modifications et des règles de zone d’un projet dans un seul fichier.                                                                                                                                                                                         |
| `city/`        | La ville : sa disposition, son stockage, sa projection sur la Terre, le document des modifications et l’importation d’un réseau.                                                                                                                                                                                       |
| `junction/`    | L’éditeur de jonction, avec sa géométrie, son dessin en 2,5D, ses arrêts et ses feux, et sa moitié de la page de comparaison.                                                                                                                                                                                          |
| `map/`         | La carte de la ville dessinée par MapLibre, le panneau Règles de zone, les vérifications du réseau, le rapport de revue de conception et la ville de la page À propos.                                                                                                                                                 |
| `place/`       | L’origine d’une ville : les régions métropolitaines, l’index des tuiles, ainsi que la récupération et le découpage des routes du lieu sur lequel s’ouvre un projet. Il n’y a pas de recherche de lieu.                                                                                                                 |
| `project/`     | Les projets : la liste de la page d’accueil, le projet dans lequel une page est ouverte (`context.rs`), les copies des aménagements partagés (`copies.rs`) et le déplacement du travail d’avant les projets (`migrate.rs`).                                                                                            |
| `restriction/` | Les règles déclarées sur une zone de la carte.                                                                                                                                                                                                                                                                         |
| `share/`       | Le partage d’un projet par un lien : le format de partage, la page de partage (créer les liens d’un projet, en ouvrir un) et ses appels au serveur.                                                                                                                                                                    |
| `shared/`      | Le noyau : catalogue, unités, ports, traductions, `Said`, le cadre de la page de comparaison (`compare.rs`, `compare_view.rs`), les scénarios d’un lieu (`scenarios.rs`), le stockage d’un projet (`scoped.rs`) et les commandes désactivées d’une page en lecture seule (`locked.rs`). Il ne dépend d’aucune tranche. |
| `shell/`       | Ce que partagent toutes les pages : le menu des réglages, les panneaux des détails et des notes, le changement de langue, le menu Se connecter (`session.rs`), le menu Partager de l’en-tête (`share.rs`) et la transformation de la copie d’un aménagement partagé en projet de la personne.                          |
| `standards/`   | Les profils de normes : des seuils locaux que seules les vérifications de la page de rue lisent à la place des seuils de base.                                                                                                                                                                                         |
| `street/`      | L’éditeur de rue, et sa moitié de la page de comparaison.                                                                                                                                                                                                                                                              |
| `sync/`        | Conserve le travail d’une personne connectée sur le serveur, quand elle l’a choisi, et le fusionne entre ses appareils.                                                                                                                                                                                                |
| `timeline/`    | Les jalons d’un projet et leur page, `timeline.html`.                                                                                                                                                                                                                                                                  |

Une tranche peut utiliser le `model` d’une autre tranche (la jonction utilise celui de la rue, la ville les deux,
`sync::local` utilise `project::model`), `city::store`, `city::binding`, `city::projection`, `city::edits` (projet,
synchronisation), `restriction::store`, `standards::model` et `standards::store` (rue), `place::area` (et
`place::Bounds`, le cadre que donne `Area::bounds` et que prend `city::projection`), `project::copies` (le partage, pour
la copie d’un aménagement partagé; le shell, pour faire d’une copie un projet de la personne), `project::context` (le
projet dans lequel une page est ouverte, et s’il est affiché en lecture seule) et `shared`. Elle n’utilise jamais le
modèle de vue, les vues ni les textes d’une autre tranche, ni ses autres modules, sauf comme le dit cette section :

- Le panneau de l’archive. La page Compte (`account/mod.rs`) le construit avec `archive::vm::ArchiveVm::for_page` et le
  monte avec `archive::view::ArchiveView`. Le menu Partager du shell construit un `ArchiveVm::for_page` pour son
  exportation (`shell/view.rs`).
- Le tiroir du shell. Le tiroir des notes de la page de rue (`street/view/notes_tray.rs`) est fait avec `shell::tray`.
- Le montage du shell. Les pages de rue, de jonction, de carte et de compte montent le shell avec `shell::mount`, en
  indiquant avec `shell::NotesAtStart` comment leurs notes s’ouvrent. Sa cible est un `shell::Target`, que mettent en
  œuvre `StreetVm`, `JunctionVm` et `MapVm`; la page Compte et une jonction qui ne peut pas être dessinée lui donnent un
  `shell::Bare`.
- Le dessin d’une zone. La carte utilise `restriction::draw`, la machine à états pure qui dessine le contour d’une zone
  (`map/vm/mod.rs`, `map/vm/areas.rs`, `map/view/mod.rs`, `map/view/areas_view.rs`).
- Le rapport. `map/vm/report.rs` dessine ses figures avec `street::view::drawing` et `junction::view::figure`.
- Un test du shell, qui construit un `street::vm::StreetVm`.

De `street/`, `junction/` et `map/`, une autre tranche n’utilise que ce qui se trouve sous `model/`, sauf pour le
rapport et ce test du shell.

Les tests utilisent aussi davantage des autres tranches, pour mettre en place ce qu’ils testent :

- `project::ProjectStore` et `project::store::PROJECTS_KEY` : les tests de l’archive, de la ville, de la jonction, de
  la carte, du partage, du shell, de la rue et de la synchronisation.
- `restriction::fixtures` : les tests de la ville, de la carte et de la rue.
- `shell::view::i18n_keys` : les tests du compte, de la jonction, de la carte et des projets.
- `sync::SyncVm` et `sync::Status` : les tests du compte.
- `sync::tests::Device`, `sync::local::ids`, `share::links::ProjectShares`, `share::store` et `timeline::store` : les
  tests des projets.
- `city::scenario::Action` : les tests de la rue et de la jonction.

Quelques magasins sont aussi utilisés d’une tranche à l’autre. `sync::local` utilise `project::store` et
`project::migrate::{legacy_waiting, legacy_kept, area_moved}`. `project::store` utilise `city::edits`, `city::store` et
`restriction::store`. `sync::local` et `project::store` utilisent `timeline::store`. La page Compte (`account/mod.rs`)
exécute `project::migrate::move_legacy_work` avant de synchroniser.

Les magasins et les éditeurs utilisent aussi `sync::model` (`Id`, `Kind`, `Change`, `Origin`, `Report`, `mark`), qui ne
dépend que de `shared`. Le shell construit `sync::SyncVm`; toute autre tranche n’accède à la synchronisation que par
`sync::hub` et `sync::model`. `shared` ne dépend d’aucune tranche.

Une page faite de plusieurs tranches est assemblée dans `lib.rs`, la racine de composition, qui peut utiliser toute
tranche (voir [Pages et points d’entrée](#pages-et-points-dentrée)).

## Couches dans une tranche

Une tranche contient, selon ses besoins, ces couches :

- Le modèle : les règles d’édition et les calculs purs sur le domaine, sans Leptos et sans texte. Dans les tranches de
  la rue et de la jonction, il n’a aucun DOM, aucun signal et aucune entrée-sortie. Une modification renvoie un `bool`,
  et un refus en donne la raison.
- Le modèle de vue : il expose des accesseurs réactifs prêts à présenter et reçoit des commandes. Il n’atteint le
  navigateur que par les [ports](#ports).
- Les vues : des vues Leptos qui se lient à un modèle de vue et lui transmettent les événements. Elles ne décident de
  rien.

Dans les trois plus grandes tranches, `street/`, `junction/` et `map/`, chaque couche est un dossier : `model/`, `vm/`
et `view/`. Seuls `mod.rs` (les points d’entrée), `i18n/` et `CLAUDE.md` restent à la racine de la tranche;
`layout_guard.rs` échoue sinon. Dans les tranches de la rue et de la jonction, `vm/` contient aussi les mots destinés à
un lecteur d’écran (`text.rs`), les machines à états du clavier (`keys.rs`) et la poignée que prennent les vues
(`watch.rs`); dans la tranche de la carte, il contient les modèles de vue secondaires, la caméra, le rapport et ce qui
est construit pour la carte. `view/` contient les vues Leptos, le dessin SVG et les tests des composants
(`view/tests.rs`).

Les autres tranches gardent leurs fichiers côte à côte à leur racine : `model.rs`, `vm.rs`, `view.rs`, `text.rs`,
`tests.rs` et `i18n/`, selon leurs besoins. Le shell garde son `vm.rs`, ses vues Leptos et `tests.rs` côte à côte.

Le dessin est produit par des fonctions pures qui renvoient des chaînes SVG, placées avec `inner_html`
(`street/view/iso/`, `junction/view/iso/`, et `map/view/svg.rs` pour la seule ville de la page À propos). La logique du
pointeur et du clavier est faite de machines à états pures et testées. La page Carte fait exception : elle dessine avec
MapLibre, sans SVG.

## Pages et points d’entrée

Chaque page de `web/` a un script qui charge le module de l’éditeur et appelle son point d’entrée :

| Page                | Script        | Point d’entrée                                                                                  |
| ------------------- | ------------- | ----------------------------------------------------------------------------------------------- |
| `index.html`        | `home.js`     | `mount_projects` (`lib.rs`) : les projets de la personne.                                       |
| `about.html`        | `about.js`    | `mount_about` (`lib.rs`), puis `prepare_city` et `mount_city` : À propos de CityLoom.           |
| `map.html`          | `map.js`      | `mount_map` (`map/mod.rs`).                                                                     |
| `street.html`       | `app.js`      | `open_street` et `mount_street_page` (`street/mod.rs`), `mount_standards` (`lib.rs`).           |
| `intersection.html` | `junction.js` | `enter_junction`, `open_junction` et `mount_page` (`junction/mod.rs`).                          |
| `account.html`      | `account.js`  | `mount_account` (`account/mod.rs`).                                                             |
| `timeline.html`     | `timeline.js` | `mount_timeline` (`lib.rs`) : les jalons d’un projet.                                           |
| `share.html`        | `share.js`    | `mount_share` (`lib.rs`) : les liens de partage d’un projet, ou un lien de partage qu’on ouvre. |
| `compare.html`      | `compare.js`  | `mount_compare` (`lib.rs`) : deux conceptions d’une même rue ou jonction.                       |

`credits.html` ne charge aucun module. Chaque script monte ensuite le shell (`mount_shell`). Les pages qui ouvrent la
ville (`about.js`, `map.js`, `app.js`, `junction.js` et `compare.js`) appellent `prepare_city` (`place/mod.rs`) avant de
l’ouvrir : cette fonction obtient les routes de la zone où la personne travaille, à moins qu’elles ne soient déjà
conservées.

`lib.rs` est la racine de composition, où les tranches peuvent se rencontrer. La page d’accueil est
`project::mount_list`, à qui l’on remet `share::links::ProjectShares` de la tranche du partage comme
`project::model::ProjectLinks`, pour que les liens de partage d’un projet prennent fin avant lui. La page À propos
exécute `project::migrate::move_legacy_work`, puis `map::mount_about`. Les pages de la chronologie, du partage et de
comparaison y sont aussi assemblées, et `i18n_for` y inscrit les messages de chaque tranche.

Une page d’un projet s’ouvre dans celui-ci (`project::context::open_page`) : le projet que son adresse nomme avec
`?project=<id>`, sinon celui ouvert en dernier. S’il n’y en a aucun, la page va à `index.html?missing=1`. Les liens
vers les pages d’un projet portent `project=<id>` (`shared::project_id::link`).

Un projet s’ouvre en lecture seule quand son adresse contient `&view=1`, ou quand c’est la copie d’un aménagement
partagé (`project::context::read_only`). Le stockage de la page refuse alors toute écriture de l’aménagement
(`ScopedStorage::only_to_read`, que signale `Storage::read_only`), le `read_only()` de chaque modèle de vue refuse les
modifications, et la barre du haut du shell l’indique. Les liens entre les pages d’un projet gardent `view=1`.

La page de comparaison, `compare.html?street=<edge>` ou `compare.html?junction=<node>`, s’ouvre par le lien Comparer du
panneau des scénarios d’une rue ou d’une jonction. Elle montre côte à côte deux conceptions du lieu (la référence ou un
scénario), avec ce qui diffère dans la seconde. Elle est en lecture seule : `mount_compare` ne fait que consulter le
projet (`project::context::peek`) et n’écrit rien. La tranche de la rue ou de la jonction dessine sa moitié
(`mount_compare_street`, `mount_compare_junction`) dans le cadre de `shared/compare_view.rs`.

La page de partage, `share.html?project=<id>`, crée les liens de partage d’un projet : la tranche `share/` compose un
instantané du projet et l’envoie à `/api/share` sur le serveur. Un lien, `share.html#<token>`, ouvre un aménagement
partagé : la page récupère l’instantané, le conserve comme une copie (`project::copies`) qui n’est jamais synchronisée
et n’est pas un projet de la personne, et ouvre sa carte en lecture seule. [Partager un aménagement](../sharing.md)
indique ce qui est conservé, pour qui et pendant combien de temps.

## Ports

Un modèle de vue n’atteint le navigateur que par des ports. Ce sont les champs de `Ports`, dans `shared/ports.rs` :

- `Navigator` : ouvre une autre page du site.
- `PdfWriter` : écrit la revue de conception en PDF.
- `Downloads` : donne à une personne un fichier à garder.
- `Fetcher` : demande quelque chose à un serveur (HTTP).
- `Announcer` : dit quelque chose à une personne qui ne voit pas la page.
- `Storage` : conserve des chaînes d’une visite à l’autre.
- `Scheduler` : exécute quelque chose après un délai.
- `Mapper` : la carte sur laquelle la page montre la ville.
- `Page` : la page elle-même : son adresse, la langue du navigateur, le presse-papiers.
- `Account` : le compte de la personne sur le serveur.
- `WorkSync` : le travail de la personne sur le serveur.
- `ArchiveFiles` : une archive de conception remise sous forme de fichier, et un fichier choisi.
- `ShareApi` : les aménagements partagés du serveur (`/api/share`).

`shared/ports.rs` contient aussi leurs doublures, et `test_ports()` et `test_ports_with_time()` en donnent un ensemble :
c’est ainsi que les modèles de vue sont testés en natif, sans navigateur. `shared/platform.rs` en est le côté
navigateur : `browser_ports()` et `browser_ports_in(project)`, dont le stockage est celui du projet.

## Traduction

Le modèle ne contient jamais de texte. Il exprime les choses par des données `Said` (une clé de message et des
arguments) que la langue courante met en mots; un historique suit donc un changement de langue. Chaque tranche qui
possède des messages a `i18n/en.ftl` et `i18n/fr.ftl` (Fluent), et un test échoue quand les deux diffèrent. Les termes
français sont dans `docs/glossary-fr-ca.md`.

## Modules WebAssembly

Le navigateur exécute deux modules WebAssembly, tous deux pilotés par `wasm-bindgen` :

- l’éditeur, `cityloom_editor`, que charge chaque page sauf `credits.html`;
- le générateur de PDF, `cityloom_report_pdf`, chargé seulement la première fois qu’un rapport est demandé, pour que sa
  lourde dépendance (krilla) reste hors du module de l’éditeur. L’éditeur lui remet le rapport sous forme de données
  simples (`cityloom_report`).

Le réseau de rues est préparé à l’avance, et non dans le navigateur : `citymodel_builder` exécute
[osm2streets](https://github.com/a-b-street/osm2streets) une seule fois sur toute la région métropolitaine et découpe le
réseau (`cityloom_model`) en tuiles JSON que l’éditeur télécharge et dans lesquelles il découpe un lieu. Seul
`citymodel_builder` dépend d’osm2streets, derrière sa fonctionnalité `import`; osm2streets n’atteint donc jamais le
navigateur. Le serveur, `cityloom_server`, n’est pas construit en WebAssembly.
[Organisation du dépôt](../project-layout.md#les-caisses) énumère toutes les caisses.

`city/import.rs` transforme un réseau en disposition de la ville : les routes deviennent des rues, et la rencontre de
trois à cinq branches que l’éditeur de jonction sait dessiner devient une jonction.

## Comptes et serveur

On peut se connecter avec OpenStreetMap, Google, Microsoft ou Apple (OpenID Connect, code d’autorisation avec PKCE).
Une identité est le fournisseur et son sujet; un courriel ne sert jamais à trouver un compte ni à en joindre deux. Les
fournisseurs s’associent à la main, depuis la page Compte.

Tout ce que le serveur conserve sur une personne est chiffré : chaque champ est scellé avec AES-256-GCM sous une clé
propre à cette personne, elle-même scellée sous une clé maîtresse que détient l’environnement du serveur, et liée à sa
table, à sa colonne et à sa ligne. Les identités se retrouvent par un HMAC du fournisseur et du sujet. La suppression
d’un compte en retire aussitôt les lignes; les copies dans les sauvegardes disparaissent à l’expiration de celles-ci.
Sans connexion, le site fonctionne exactement comme avant, et il fonctionne encore quand le volet des comptes du
serveur est indisponible.

Le serveur conserve aussi les aménagements partagés à `/api/share` : l’instantané qu’ouvre un lien de partage, conservé
90 jours, comme l’explique [Partager un aménagement](../sharing.md).

## Modifications dans le nuage

Une personne qui a choisi « Stocker mon travail sur le serveur » voit le travail de chacun de ses projets conservé sous
son compte : les projets eux-mêmes, leurs modifications de rues et de jonctions, leurs règles de zone, leur zone
choisie et leurs jalons. Chaque appareil sur lequel elle se connecte les reçoit. Le stockage du navigateur reste la
copie de travail : les éditeurs fonctionnent hors ligne et rattrapent leur retard au retour du réseau. Chaque lieu est
un enregistrement dont la révision vient du compteur du compte; un appareil demande ce qui dépasse la dernière révision
qu’il a vue et envoie ce qu’il a modifié. Quand deux appareils ont modifié le même lieu, la modification la plus
récente l’emporte sur les deux, et l’appareil dont la modification a perdu le dit. Le contenu des enregistrements est
chiffré comme tout ce que le serveur conserve, lié à son genre, à sa clé et à son compte; les clés des lieux
(identifiants OpenStreetMap), les identifiants de règles et l’heure des modifications ne le sont pas, parce que le
serveur les compare et les trie. Cesser le stockage supprime la copie sur le serveur et garde celle du navigateur;
supprimer le compte supprime aussi la copie sur le serveur. Les préférences (thème, langue, panneaux) restent propres à
chaque appareil.

## JavaScript

La logique appartient à Rust. Les scripts de page dans `web/` (`home.js`, `about.js`, `map.js`, `app.js` pour la page de
la rue, `junction.js`, `account.js`, `timeline.js`, `share.js` et `compare.js`) servent au démarrage : ils chargent le
module, appellent `mount_*` et montent le shell. Les scripts de la rue, de la jonction et de la comparaison préparent
aussi les motifs de hachures; ceux de la rue et de la jonction enregistrent les modifications de la page à `pagehide`,
et `map.js` crée la carte. `boot.js` affiche un avis quand le module d’une page ne peut pas être chargé ou démarré,
`city.js` lit pour quel lieu une page a été ouverte, et `report.js` charge le générateur de PDF. La seule exception à la
règle est `web/mapcanvas.js`, l’adaptateur qui possède la carte MapLibre.

## Persistance

Une page ouverte depuis la carte modifie un lieu de la ville. Ses modifications sont réécrites par `CityStore`, avec un
délai antirebond de 250 ms, et enregistrées quand on quitte la page. Chaque projet garde ses modifications dans un
document à lui, `cityloom-edits@<project>` (`shared/scoped.rs`), dont les enregistrements sont indexés d’après ce dont
le lieu a été fait (identifiants de nœuds et de voies OpenStreetMap); une modification suit donc son lieu dans une
autre zone qui le contient. En route vers le serveur, les enregistrements du premier projet gardent leurs clés nues, et
ceux de tout autre projet partent sous la forme `<project>:<key>`.

## L’archive de conception

La page Compte peut exporter le travail de conception d’un projet (celui ouvert en dernier) dans un seul fichier JSON,
et y importer un tel fichier. Le format est décrit dans `archive/model.rs`; il porte un `format_version` entier (1) et
un `kind` valant `cityloom-design`, la version de l’application, l’heure, ce sur quoi la conception a été faite (zone,
somme de contrôle de la tuile, version de l’importeur), le document `cityloom-edits` (lu à partir de la version 4 et
écrit à la version que son contenu exige) et le document `cityloom-restrictions`. Il ne contient ni les jalons du
projet, ni le document de synchronisation, ni rien du compte, ni la langue et le thème, ni les réseaux de rues
téléchargés. Le fichier se nomme `*.cityloom-design.json`, jamais un simple `.citymodel`, qui est l’archive de zone que
construit `citymodel_builder`.

Une importation vérifie tout le fichier avant d’écrire quoi que ce soit. Un fichier qui n’est pas du JSON, qui n’est pas
une archive de conception CityLoom, qui est endommagé, qui dépasse 10 Mo, qui vient d’un format plus récent ou qui
contient des modifications ou des règles d’une version plus récente est refusé avec un message qui le dit et qui précise
que rien n’a été modifié. Les lieux absents de l’appareil sont ajoutés; un lieu présent qui diffère n’est remplacé que si
la personne choisit « Utiliser ceux de l’archive » pour cette importation. Des modifications ou des règles de l’appareil
qui sont endommagées ou d’une version plus récente arrêtent l’importation. Les champs inconnus sont ignorés à la
lecture, et `notes` et `scenarios` sont des noms réservés que la version 1 n’écrit ni ne lit. Les enregistrements
importés sont marqués pour la synchronisation comme toute modification.
