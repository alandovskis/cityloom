# Glossaire anglais / français (Canada)

The French the app speaks (fr-CA), for translators and reviewers. The messages are in `src/shell/i18n/fr.ftl` and `src/street/i18n/fr.ftl`. One term per English word, used everywhere: add a row when a screen needs a new word, and do not translate the same word two ways. The terms in the last section are not settled.

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
