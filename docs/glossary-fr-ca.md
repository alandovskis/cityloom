# Glossaire anglais / français (Canada)

The French the app speaks (fr-CA), for translators and reviewers. The messages are in `src/shell/i18n/fr.ftl`, `src/street/i18n/fr.ftl`, `src/city/i18n/fr.ftl`, `src/map/i18n/fr.ftl`, `src/place/i18n/fr.ftl` and `src/junction/i18n/fr.ftl`. One term per English word, used everywhere: add a row when a screen needs a new word, and do not translate the same word two ways. The terms in the last section are not settled. MapLibre’s own control strings, the basemap’s labels, OSM data and `web/credits.html` stay English.

## Style rules

- Typography follows the OQLF: a non-breaking space before `:` and inside `« »`, nothing before `;`, `!` or `?`. Write ordinary spaces in the `.ftl` files; the engine makes the non-breaking ones. Numbers use a comma for the decimal (`2,50 m`) and a non-breaking space before the unit and between thousands.
- Quotation marks and apostrophes: `’` (typographic), `« »` for quotations.
- Days are lowercase with a period: `lun. mar. mer. jeu. ven. sam. dim.`
- Plurals use the CLDR rule for French: `0` and `1` are singular. Write them with selectors (`[one]` / `*[other]`).
- Gender: avoid it where a sentence can. Labels such as `Ajout : …` and `réussite` / `échec` are nouns, so they agree with any name that follows.
- `élément` is the word for a piece of a street (not `pièce`, not `segment`). `Réorganiser` is the word for Arrange.
- `Vélo` is deliberately both the mode (`mode-bike`) and its group of pieces (`group-cycling`): one idea, one word.
- Road vocabulary: `Accotement`, `Autoroute`, `Chaussée` and `cédez le passage` are the forms chosen as Québec usage; a Québec reviewer should confirm them (see below).

## Terms

| Where | English | Français (fr-CA) | Note |
| --- | --- | --- | --- |
| Street page | piece (of a street) | élément |  |
| Street page | Arrange | Réorganiser |  |
| Street page | Sidewalk | Trottoir |  |
| Street page | Planting strip | Bande plantée |  |
| Street page | Planted median | Terre-plein central planté |  |
| Street page | Bike lane | Piste cyclable | The catalogue's lane may have a curb, so a track, not a painted `bande cyclable`. |
| Street page | Driving lane | Voie de circulation |  |
| Street page | Transit lane | Voie réservée au transport en commun | Shorter form for buses only: `voie réservée aux autobus`. |
| Street page | Parking | Stationnement |  |
| Street page | Loading zone | Zone de chargement |  |
| Street page | Shoulder | Accotement |  |
| Street page | Bike rack | Support à vélos |  |
| Street page | Bikeshare station | Station de vélos en libre-service |  |
| Street page | Utility pole | Poteau de services publics |  |
| Street page | Bus shelter | Abribus |  |
| Street page | Bus station | Station d’autobus |  |
| Street page | Bench | Banc |  |
| Street page | Café terrace | Terrasse de café |  |
| Street page | Street lamp | Lampadaire |  |
| Modes and groups | Walking | Marche |  |
| Modes and groups | Biking / Cycling | Vélo |  |
| Modes and groups | Transit | Transport en commun |  |
| Modes and groups | Cars and trucks | Autos et camions |  |
| Modes and groups | Greenery | Verdure |  |
| Modes and groups | Roadway | Chaussée |  |
| Modes and groups | Furniture | Mobilier urbain |  |
| Modes and groups | Utilities | Services publics |  |
| Street page notes | Use (a column of the width table) | Usage |  |
| Street page notes | Today | Aujourd’hui |  |
| Street page notes | Your design | Votre aménagement |  |
| Street page notes | Change (a column) | Variation | To confirm: see "Variation N" below. |
| Street page notes | Step | Étape |  |
| Street page notes | check (of the street) | vérification | `0` and `1` are singular, as in CLDR. |
| Street page notes | passes / fails | réussite / échec | Nouns, so they agree with any check's name. |
| Street page notes | Set at a junction | Se règle à une intersection |  |
| Street page notes | Not modelled | Non modélisé |  |
| Street page frame | City map | Carte de la ville |  |
| Street page frame | Street cross-section | Coupe transversale de la rue |  |
| Street page frame | 12 m wide | 12 m de large |  |
| Street page frame | between A and B | entre A et B |  |
| Street page frame | Start over | Recommencer |  |
| Street page frame | Undo / Redo | Annuler / Rétablir |  |
| Street page frame | Add a piece | Ajouter un élément |  |
| Street page frame | Time of day | Heure de la journée |  |
| Street page frame | except (clock note) | sauf |  |
| Street page frame | Mo Tu We Th Fr Sa Su | lun. mar. mer. jeu. ven. sam. dim. | Lowercase, with a period (fr-CA). |
| Street page frame | Got it | Compris |  |
| Street page frame | Changes made | Modifications apportées |  |
| Street page frame | As imported | Tel qu’importé |  |
| Street page frame | Street cross-section editor | Éditeur de coupe transversale de la rue |  |
| Transit priority measures | Transit Streets / Transit Ways | Rues / Voies de transport en commun | The 30 Atlas names are the `atlas-<code>-name` messages. |
| Transit priority measures | Center-Running Transit Lanes | Voies centrales réservées au transport en commun |  |
| Transit priority measures | Edge-Running / Curb-Adjacent | latérales / en bordure de trottoir |  |
| Transit priority measures | Contraflow | à contresens |  |
| Transit priority measures | Queue-Jump Lane | Voie de dépassement de file |  |
| Transit priority measures | Bus Bulbs | Avancées de trottoir pour autobus |  |
| Transit priority measures | Transit Modal Filter | Filtre modal pour le transport en commun |  |
| Transit priority measures | Yield-Controlled Bus Gates | Portes d’autobus à cédez le passage | Chosen form, to confirm (see below). |
| Piece panel | Surface / Planting (heading) | Revêtement / Plantation |  |
| Piece panel | Other times | Autres périodes |  |
| Piece panel | Two-way | Double sens |  |
| Piece panel | Curb | Bordure |  |
| Piece panel | Wider by / Narrower by | Élargir de / Rétrécir de | Button labels, so verbs. |
| Piece panel | Allowed 2.50 to 4.00 m | Valeurs permises : de 2,50 à 4,00 m |  |
| Piece panel | Vehicle: Bus / Tram | Véhicule : autobus / tramway |  |
| Piece panel | Away from you / Toward you | S’éloigne de vous / Vient vers vous |  |
| Piece panel | Asphalt, Concrete, Brick pavers, Gravel | Asphalte, Béton, Pavés de brique, Gravier | The materials are `material-<id>`, the curbs `curb-<id>`. |
| The drawing of the section | Street edge | Limite de la rue |  |
| The drawing of the section | Unused | Inutilisé |  |
| The drawing of the section | 1.0 m too wide | 1,0 m de trop |  |
| The drawing of the section | Street width | Largeur de la rue |  |
| The drawing of the section | Scale | Échelle |  |
| The drawing of the section | Change 2 | Variation 2 | As the column in the changes table; to confirm, see below. |
| The drawing of the section | segment (a piece, in the label) | élément |  |
| The drawing of the section | Piece marks (SW, TL…) | TO, VC, VR, ST, TP, ZC, AC, PC, PL, SV, VP, PO, AB, SA, BC, TE, LA | Two letters, one per kind (`kind-mark-<id>`), unique among themselves. |
| Changes list and announcements | Add / Remove / Move / Resize | Ajout / Retrait / Déplacement / Redimensionnement | Nouns followed by ` : ` and the lowercase name of the kind, so no article (and no gender) is needed. |
| Changes list and announcements | Parking surface: permeable paving | Stationnement, surface : revêtement perméable | The kind, then what changed. |
| Changes list and announcements | Curb: none | Bordure : aucune |  |
| Changes list and announcements | Direction: two-way | Sens : double sens | As `Two-way` above. |
| Changes list and announcements | Parking is transit lane 07:00-10:00 | Stationnement devient voie réservée au transport en commun de 07:00 à 10:00 |  |
| Changes list and announcements | Remove other times from … | Retrait des autres périodes : … | `Other times` is `Autres périodes`. |
| Changes list and announcements | Reset to existing | Retour à l’état existant |  |
| Changes list and announcements | Earlier changes | Modifications antérieures |  |
| Changes list and announcements | Traffic keeps right / left | La circulation se fait à droite / à gauche |  |
| Changes list and announcements | Fits the street width | Respecte la largeur de la rue | A check's label. |
| Changes list and announcements | Sidewalk on both sides | Trottoir des deux côtés |  |
| Changes list and announcements | Room for emergency vehicles | Place pour les véhicules d’urgence |  |
| Changes list and announcements | Freeway | Autoroute | Chosen as the Québec road term, to confirm; the measures that are `Freeways only` are `Autoroutes seulement`. |
| City names | A, B and N more | `A, B et N autres` | `1 autre` for one. |
| City names | Edge of the map | `Limite de la carte` | In a sentence: `la limite de la carte`. |
| City names | Kind · between here and there | `Genre · entre A et B` | `entre … et …` needs no contraction with `le bout de`, `un raccordement sur`, `la limite de`. `à travers la ville` for an edge between two non-junctions. |
| City names | Junction today / Earlier changes | `Jonction aujourd’hui` / `Modifications antérieures` | The junction's default history labels. |
| Map page | place (a street or a junction to open) | lieu | `0` and `1` are singular: `1 lieu`, `2 lieux`. |
| Map page | Junctions / Streets (headings) | Jonctions / Rues |  |
| Map page | 3 streets (at a junction) | 3 rues |  |
| Map page | Works / Still works | Fonctionne / Fonctionne toujours |  |
| Map page | Changed (a place's tag) | Modifié | The title block's count is `Modifiés`. |
| Map page | Needs attention | À corriger | To confirm, see below. |
| Map page | Every check passes | Toutes les vérifications réussissent | As `vérification` / `échouent` on the street page. |
| Map page | Key to the map | Légende de la carte |  |
| Map page | basemap | fond de carte |  |
| Map page | Whole city | Toute la ville |  |
| Map page | Zoom in / Zoom out | Zoom avant / Zoom arrière |  |
| Map page | Search places | Rechercher des lieux |  |
| Map page | 12 places match · showing the first 8 | 12 lieux correspondent · 8 premiers affichés |  |
| Map page | as first laid out | telle que tracée au départ | To confirm, see below. |
| Map page | Press again to start over | Appuyez de nouveau pour recommencer | `Recommencer` as on the street page. |
| Map page | Editors (the navigation) | Éditeurs |  |
| Map page | Sheet tools | Outils de la feuille | To confirm, see below. |
| Place search (home page) | Find a place | Chercher un lieu | `lieu`, as on the map page. |
| Place search (home page) | Find / Open (the button) | Chercher / Ouvrir |  |
| Place search (home page) | 2 places found | 2 lieux trouvés | `0` and `1` are singular: `1 lieu trouvé`. |
| Place search (home page) | Use the arrow keys, then Enter | Utilisez les flèches, puis Entrée | `Entrée` as the key's name (`ui-key-enter`). |
| Place search (home page) | Getting the streets of X… | Chargement des rues de X… | X is the place's own name, so no contraction (`de Le Plateau` is possible and left as it is). |
| Place search (home page) | the roads … could not be fetched | les rues … n’ont pas pu être obtenues | The roads are `rues`, as everywhere in the app. The error in brackets is the browser's own, not translated. |
| Place search (home page) | storage (the browser's) | stockage |  |
| Place search (home page) | tile index | index des tuiles | Never shown to a person today: the loader goes on without it. |
| Home page | Redesign the streets of your city | Réaménagez les rues de votre ville | `réaménager` for redesigning a street, `réorganiser` for Arrange. |
| Home page | rearrange (a street) | réorganiser | As Arrange on the street page. |
| Home page | Open the city map | Ouvrir la carte de la ville |  |
| Home page | home (the brand link) | accueil |  |
| Home page | Streets © OpenStreetMap contributors | Rues © les contributeurs d’OpenStreetMap | OpenStreetMap's own French credit is `© les contributeurs d’OpenStreetMap`. |
| Home page | placeholders (the rules the checks use) | provisoires | As the map page's `exemples`; to confirm, see below. |
| Junction | junction | jonction | As in the city's names (`Jonction 4`); `intersection` only in the Atlas name `L2`. |
| Junction | arm (one street at a junction) | branche | Not shown alone: an arm is said `Rue (nord)`. |
| Junction | north, north-east … (in a sentence) | nord, nord-est, est, sud-est, sud, sud-ouest, ouest, nord-ouest | Lowercase in brackets after the street: `Rue Principale (nord)`. |
| Junction | N, NE, E, SE, S, SW, W, NW (tags) | N, NE, E, SE, S, SO, O, NO |  |
| Junction | control (how a junction is run) | signalisation | `Signalisation : feux de circulation` in the history. |
| Junction | No control | Aucune signalisation | To confirm, see below. |
| Junction | Side streets stop | Arrêt sur les rues secondaires | To confirm, see below. |
| Junction | All-way stop | Arrêt toutes directions |  |
| Junction | Traffic signal | Feux de circulation | Plural, as in Québec; lowercase in a sentence. |
| Junction | Roundabout | Carrefour giratoire |  |
| Junction | crossing (for people on foot) | passage pour piétons |  |
| Junction | refuge island | îlot refuge |  |
| Junction | bulb-out (curb extension) | avancée de trottoir | As the Atlas's bus bulb (`Avancée de trottoir pour autobus`). |
| Junction | corner radius / Corner after X | rayon / Coin après X |  |
| Junction | bearing | orientation |  |
| Junction | offset (of an arm) | décalage |  |
| Junction | lane (driving) | voie |  |
| Junction | bus lane | voie réservée aux autobus | Across a roundabout's middle: `voie d’autobus par le centre`. |
| Junction | cycle track | piste cyclable |  |
| Junction | turn / left turn / straight on / right turn | virage / virage à gauche / tout droit / virage à droite |  |
| Junction | allowed / not allowed (a turn) | permis / interdit |  |
| Junction | add / remove (at the end of a history label) | ajout / retrait | Nouns, as on the street page. |
| Junction | approach measure | mesure d’approche |  |
| Junction | turn management | gestion des virages |  |
| Junction | dead end | cul-de-sac |  |
| Junction | Right-in/right-out | Entrée et sortie à droite seulement | As the Atlas name `L3`; shortened to `Entrée et sortie à droite` where it says why a turn is blocked. |
| Junction | queue jump (G1 to G3) | voie de dépassement de file | Singular forms of the Atlas names: `Voie décalée de dépassement de file`, `… en bordure de trottoir`, `Voie virtuelle …`. |
| Junction | bus gate (H1, H2) | porte d’autobus | `à feux de signalisation`, `à cédez le passage`, as the Atlas names. |
| Junction | Signal-protected on-street platform | Quai sur rue protégé par des feux |  |
| Junction | Crossing distance | Distance de traversée | A check's label; the other check labels are whole sentences (`Les voies couvrent chaque virage`). |
| Junction | Sidewalk survives the corner / eats the sidewalk | Le trottoir survit au coin / gruge le trottoir | `gruger` is Québec usage; to confirm, see below. |
| Junction | One check needs attention: … | Une vérification est à corriger : … | As the map's `À corriger`. |
| Junction | Started over from the junction as it is today | Retour à la jonction telle qu’elle est aujourd’hui |  |
| Junction | Points at banned turns: … | Voies menant à des virages interdits : … | A check's failing detail; the lanes are its subject. |
| Junction page | Junction plan | Plan de la jonction | As the map page's `Ouvre le plan de la jonction`. |
| Junction page | Junction plan editor (the plan's name) | Éditeur du plan de la jonction |  |
| Junction page | Edited / As imported (title block) | Modifiée / Telle qu’importée | Feminine, agreeing with `jonction`; the street page's are `Modifié` / `Tel qu’importé`. |
| Junction page | Data (title block) | Données |  |
| Junction page | To cross / Lanes in | À traverser / Voies entrantes | Columns of the Getting across table; a street with no crossing says `aucun` (a `passage`). |
| Junction page | Crossing / Merging / Splitting / All (conflict points) | Croisement / Convergence / Divergence / Total |  |
| Junction page | path (of a turn) | trajectoire |  |
| Junction page | In use on N, E | Utilisée sur N, E | Agrees with `mesure`. |
| Junction page | Set on a street (a measure) | Se règle sur une rue | One of this junction's streets, in the panel beside the plan. |
| Junction page | Street editor | Éditeur de rue | Short form of `Éditeur de coupe transversale de la rue`. |
| Junction page | Not modelled (a measure) | Non modélisée | Agrees with `mesure`; the street page's column is `Non modélisé`. |
| Junction page | Junction today | Jonction aujourd’hui | As the city's history label. |
| Junction plan | Plan of the junction, north up | Plan de la jonction, nord en haut |  |
| Junction plan | N · 20.0 m road | N · chaussée de 20,0 m | `· décalée de 0,5 m` when shifted (agrees with `chaussée`). |
| Junction plan | Bus only (road marking) | Autobus seulement |  |
| Junction plan | Turn X (a grip) | Faire pivoter X |  |
| Junction plan | Change the corner radius / Move the crossing | Modifier le rayon du coin / Déplacer le passage pour piétons |  |
| Junction page | Details (the left panel's button) | Détails |  |
| Junction page | Streets / Measures (notes tabs) | Rues / Mesures | Checks and Changes are the shell's `Vérifications` / `Modifications`. |
| Junction page | Where paths meet | Où les trajectoires se rencontrent |  |
| Junction page | Key to the strips (the plan's legend) | Légende des bandes |  |
| Junction page | turn a street (keyboard list) | faire pivoter une rue | As the plan's grip. |
| Junction inspector | Junction control (a heading) | Signalisation de la jonction |  |
| Junction inspector | Smaller / Larger / Narrower / Wider / Tighter / Closer / Farther / Shorter / Longer by 0.5 m (steppers) | Réduire / Agrandir / Rétrécir / Élargir / Resserrer / Rapprocher / Éloigner / Raccourcir / Allonger de 0,5 m | Verbs, as the street page's `Élargir de` / `Rétrécir de`, so nothing agrees with a gender. |
| Junction inspector | Shift left / right by 0.1 m | Décaler de 0,1 m vers la gauche / la droite |  |
| Junction inspector | Turn anticlockwise / clockwise by 5° | Faire pivoter de 5° dans le sens antihoraire / horaire | As the plan's grip, `Faire pivoter`. No space before `°` (an angle). |
| Junction inspector | 1.0 m to 15.0 m (a field's range) | De 1,0 m à 15,0 m |  |
| Junction inspector | Size of the roundabout / m across | Taille du carrefour giratoire / m de diamètre |  |
| Junction inspector | Track width / Track around the outside | Largeur de la piste / Piste tout autour, à l’extérieur |  |
| Junction inspector | Bus lane through the middle / No bus lane | Voie d’autobus par le centre / Aucune voie d’autobus | As the selection's `Voie d’autobus par le centre`. |
| Junction inspector | the ring (of a roundabout) | l’anneau |  |
| Junction inspector | Curb radius | Rayon de la bordure |  |
| Junction inspector | Lane 2 of 2 / the only lane / nearest the middle / nearest the curb | Voie 2 sur 2 / la seule voie / la plus près du centre / la plus près de la bordure |  |
| Junction inspector | Left / Straight on / Right to X (where a lane goes) | À gauche / Tout droit / À droite vers X |  |
| Junction inspector | Where this lane goes / Select the whole street | Où mène cette voie / Sélectionner toute la rue |  |
| Junction inspector | Set back from the junction | Retrait par rapport à la jonction | As the history's `en retrait de`. |
| Junction inspector | 2 stages of 9.0 m / 11.4 m to cross | 2 traversées de 9,0 m / 11,4 m à traverser |  |
| Junction inspector | Halfway island / Refuge island in the middle | Îlot à mi-traversée / Îlot refuge au centre |  |
| Junction inspector | Left / Right curb bulge | Avancée de trottoir à gauche / à droite | As the history's bulb-out. |
| Junction inspector | Shorten the crossing | Raccourcir la traversée |  |
| Junction inspector | Transit priority / At the approach / Bus stop / Turns | Priorité au transport en commun / À l’approche / Arrêt d’autobus / Virages |  |
| Junction inspector | Transit modal filter | Filtre modal pour le transport en commun | As the Atlas name `N1`. |
| Junction inspector | Direction (of an arm) | Orientation | The glossary's bearing. |
| Junction inspector | Shift sideways | Décalage latéral |  |
| Junction inspector | Open the cross-section | Ouvrir la coupe transversale | As the map page's `Ouvre la coupe transversale de la rue`. |
| Junction inspector | Lanes coming in | Voies entrantes | As the Getting across table's column. |
| Junction inspector | Raised table | Carrefour surélevé | To confirm, see below. |

## Terms to confirm

Chosen as the most standard Québec term, but a native speaker (the owner) should confirm them; the meaning in the catalogue leaves room for another. Alternatives marked (suggestion) were proposed by a reviewer or implementer and are not from the owner's list.

| English | Chosen | Alternative |
| --- | --- | --- |
| Transit lane | `Voie réservée au transport en commun` | `Voie réservée aux autobus`, if it is only ever a bus lane |
| Bike lane | `Piste cyclable` | `Bande cyclable`, if it is only a painted lane |
| Utility pole | `Poteau de services publics` | `Poteau d’électricité`, `Poteau de service` |
| Bus station | `Station d’autobus` | `Gare d’autobus`, for a terminal |
| Bikeshare station | `Station de vélos en libre-service` | `Station BIXI` (brand-like) |
| Bike rack | `Support à vélos` | `Support à vélo` |
| Furniture (group) | `Mobilier urbain` | |
| Utilities (group) | `Services publics` | |
| Shoulder | `Accotement` | `Bas-côté` (used elsewhere in the francophonie) |
| Queue-Jump Lane | `Voie de dépassement de file` | `Voie de contournement de file` (suggestion) |
| Virtual Queue-Jump Lane | `Voie virtuelle de dépassement de file` | |
| Bus Bulbs | `Avancées de trottoir pour autobus` | `Saillies de trottoir` (suggestion) |
| Bus Gates | `Portes d’autobus` (`à feux de signalisation`, `à cédez le passage`) | |
| On-Street Platforms | `Quais sur rue` | |
| Transit Signal Priority | `Priorité aux feux pour le transport en commun` | |
| Planted (material, curb) | `Plate-bande` (material), `Planté` (curb) | |
| Change N (changes table) | `Variation N` | |
| Yield (as in Yield-Controlled Bus Gates) | `cédez le passage` | the earlier spelling `cédez-le-passage` was replaced; check with a Québec reviewer |
| Walking (mode) | `Marche` | `Déplacement à pied` (suggestion) |
| Unnamed {class} street (city names) | `Rue { $class } sans nom` | The class is an adjective: `autoroutière`, `artérielle`, `collectrice`, `locale`. A motorway has its own message, `Autoroute sans nom`, because `rue autoroutière` is not said. |
| Junction N (city names) | `Jonction N` | `Intersection` is also used in Québec; `jonction` follows the app's junction editor. |
| Main Street junction (one street) (city names) | `Jonction Rue Principale` |  |
| End of X (city names) | `Bout de X` | In a sentence: `le bout de X`. Alternatives: `extrémité de X`, `fin de X`. |
| Connection on X (city names) | `Raccordement sur X` | In a sentence: `un raccordement sur X`. |
| An unnamed {class} street, in a sentence (city names) | `une rue { $class } sans nom` | For a place on an unnamed road: `Raccordement sur une rue locale sans nom`. A motorway: `une autoroute sans nom`. |
| End of an unnamed street (city names) | `Bout d’une rue locale sans nom` | Its own message because `de` elides before `une`; in a sentence: `le bout d’une rue locale sans nom`. |
| Cannot be drawn with the streets as they are (city names) | `Ne peut pas être dessinée avec les rues telles qu’elles sont` |  |
| Needs attention (map page) | `À corriger` | `Attention requise`, `À vérifier` (suggestion) |
| as first laid out (map page) | `telle que tracée au départ` | `telle qu’aménagée à l’origine` (suggestion) |
| Sheet tools (map page, a toolbar's name) | `Outils de la feuille` | `Outils` (suggestion) |
| place (map page) | `lieu` | `endroit` (suggestion) |
| Changed (map page: a place's tag `map-changed`, `map-state-changed`; the hero drawing's `map-svg-changed`) | `Modifié` (agrees with `lieu`) | `Modifiée`, agreeing with the `rue` or `jonction` it marks (both feminine) |
| Find a place, like Kreuzberg, Berlin (home page) | `Chercher un lieu, comme Kreuzberg, Berlin` | `Chercher un lieu, par exemple Kreuzberg, Berlin` (suggestion) |
| Searching… (home page) | `Recherche en cours…` | `Recherche…` (suggestion) |
| Redesign (home page) | `Réaménagez` | `Repensez` (suggestion) |
| placeholders (home page: the widths and rules are placeholders) | `provisoires` | `des exemples`, as the map page's fine print (suggestion) |
| No control (junction) | `Aucune signalisation` | `Sans contrôle`, `Aucun contrôle` (suggestion) |
| Side streets stop (junction) | `Arrêt sur les rues secondaires` | `Arrêt sur la rue secondaire`, `Priorité à la rue principale` (suggestion) |
| control (junction: how it is run) | `signalisation` | `contrôle`, `régulation` (suggestion) |
| Junction (the editor's word) | `jonction` | `intersection`, `carrefour` (both usual in Québec) |
| eats the sidewalk (junction check) | `gruge le trottoir` | `empiète sur le trottoir` (suggestion, more neutral) |
| bulb-out (junction) | `avancée de trottoir` | `saillie de trottoir` (suggestion) |
| Signal has room (junction check) | `Les feux ont de la place` | `Assez peu de rues pour des feux` is its passing detail; `Feux possibles` (suggestion) |
| Merging / Splitting (junction conflict points) | `Convergence` / `Divergence` | `Insertion` / `Séparation` (suggestion) |
| A signal takes turns (junction note) | `Les feux alternent les mouvements` | `Les feux attribuent le passage à tour de rôle` (suggestion) |
| Turn X (junction grip) | `Faire pivoter X` | `Tourner X` (suggestion) |
| Street editor (junction measures) | `Éditeur de rue` | `Éditeur de coupe transversale` (suggestion, longer) |
| Bus only (junction road marking) | `Autobus seulement` | `Réservé aux autobus` (suggestion) |
| (one way in) (junction inspector: a street a lane cannot go to) | `(sens unique vers la jonction)` | `(sens unique entrant)` (suggestion) |
| One way out (junction inspector) | `Sens unique sortant` | `Sens unique, en sortie seulement` (suggestion) |
| Bus lane along the way in (junction inspector) | `Voie réservée aux autobus en entrée` | `Voie réservée aux autobus à l’approche` (suggestion; `À l’approche` is already the approach measures' list) |
| Halfway island (junction inspector heading) | `Îlot à mi-traversée` | `Îlot refuge` (suggestion, shorter) |
| Curb radius (junction inspector) | `Rayon de la bordure` | `Rayon de bordure`, `Rayon du coin` (suggestion) |
| Gate distance upstream (junction inspector) | `Distance de la porte d’autobus en amont` | |
| Tighter (a corner, junction stepper) | `Resserrer` | `Réduire le rayon` (suggestion) |
| Getting across (junction notes heading) | `Traverser` | `Pour traverser`, `Traversées` (suggestion) |
| Feet (the unit, `Units::word()`) | `ft`, as in English | `pi`, the OQLF symbol for `pied`. Owner decision: French shows `ft` today. |
| The toolbox of the Transit Priority Atlas, and where CityLoom models each measure (junction page) | `La boîte à outils du Transit Priority Atlas, et l’endroit où CityLoom modélise chaque mesure` | The Atlas keeps its English name, as on the street page (`d’après le Transit Priority Atlas`). |
| Raised table (junction) | `Carrefour surélevé` | `Intersection surélevée`; `carrefour` is the word of `Carrefour giratoire`, where the app's own `Jonction` names a place of the city |
