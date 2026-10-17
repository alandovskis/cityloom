---
id: issues
title: Signaler un problème
sidebar_position: 4
---

# Signaler un problème

Nul besoin d’être développeur pour signaler un problème ou proposer une idée. Choisissez
[Nouveau signalement](https://github.com/alandovskis/cityloom/issues/new/choose) (« New issue ») sur GitHub, puis le formulaire qui convient :
quelque chose est brisé, une rue ou une jonction est mal dessinée, vous n’arrivez pas à tracer l’aménagement voulu, une
vérification est erronée, ou vous avez une idée. Chaque formulaire est en français et en anglais, et vous pouvez écrire
dans la langue de votre choix.

Chaque formulaire demande sur quelle page vous étiez et quel navigateur vous utilisez. Choisissez la réponse qui s’en
rapproche le plus; « Je ne sais pas » convient aussi. Racontez ensuite, dans vos mots, ce que vous avez vu. Un lien vers
la page (copiez-le depuis la barre d’adresse) aide beaucoup.

## Comment un signalement reçoit ses étiquettes

1. **Le formulaire.** Votre choix de page et de navigateur devient aussitôt une étiquette, selon une règle qui lit
   seulement cette réponse. Rien de ce que vous écrivez dans le texte ne sert à deviner une étiquette. Le formulaire
   donne aussi au signalement sa catégorie et l’étiquette `needs-triage` (à trier).
2. **Le tri.** Une personne responsable lit le signalement, vérifie les étiquettes, puis fixe à la main la priorité et le
   statut. Elle peut vous demander des précisions.

## Le sens des étiquettes

Chaque étiquette a un nom en anglais et en français, comme `C-bug · bogue`, et sa description indique sa famille. La
première lettre est celle de la famille.

| Famille                | Attribuée par              | Sens                                                                                                                |
| ---------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `A-` Domaine           | Le formulaire, puis le tri | La partie de CityLoom : l’éditeur de rues ou de jonctions, la carte, etc.                                           |
| `C-` Catégorie         | Le formulaire              | Le genre de signalement : un bogue, une rue mal dessinée, une idée, etc.                                            |
| `O-` Navigateur        | Le formulaire              | Le navigateur ou l’appareil : Chrome, Edge, Firefox, Safari, iPhone ou iPad, Android.                               |
| `P-` Priorité          | La personne responsable    | L’urgence de la correction : critique, élevée, moyenne ou faible.                                                   |
| `S-` Statut            | La personne responsable    | Ce qui est attendu : des précisions, des étapes pour reproduire, autre chose, ou une correction dans OpenStreetMap. |
| `needs-triage`         | Le formulaire              | Personne n’a encore examiné le signalement.                                                                         |
| `regression-untriaged` | La personne responsable    | Cela fonctionnait avant, et personne n’a encore cherché pourquoi.                                                   |

Les catégories :

- `C-bug` : quelque chose est brisé.
- `C-drawn-wrong` : une rue ou une jonction ne ressemble pas à la vraie. Les rues viennent d’OpenStreetMap; la
  correction peut donc s’y faire, et la personne responsable ajoute alors `S-upstream-osm`.
- `C-design-request` : l’éditeur ne permet pas de tracer l’aménagement que vous voulez montrer.
- `C-unsafe-check` : une vérification juge un aménagement sûr alors qu’il ne l’est pas, ou l’inverse.
- `C-enhancement` : une idée. `C-question` : une question. `C-tracking-issue` : un signalement qui en suit plusieurs
  autres.

Les étiquettes sont définies dans `.github/labels.yml` du dépôt et mises à jour à partir de ce fichier.
