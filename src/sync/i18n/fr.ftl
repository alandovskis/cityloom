# Ce que dit la synchronisation (src/sync/text.rs, src/sync/vm.rs; les éditeurs disent sync-updated et sync-replaced par clé).
sync-status-synced = Synchronisé
sync-status-pending = Pas encore synchronisé
sync-status-paused = La synchronisation est suspendue
sync-updated = Mis à jour depuis un autre appareil.
sync-replaced = Votre modification de ce lieu a été remplacée par une plus récente.
sync-paused = La synchronisation est suspendue : une partie du travail enregistré dans ce navigateur provient d’une version plus récente de CityLoom ou est illisible; elle n’est ni envoyée ni modifiée.
sync-merged = Votre travail a été fusionné avec votre compte : { $sent ->
    [one] { $sent } lieu envoyé
   *[other] { $sent } lieux envoyés
  }, { $received ->
    [one] { $received } lieu reçu.
   *[other] { $received } lieux reçus.
  }
sync-too-large-record = Un lieu est trop volumineux pour le serveur (64 Ko); il reste sur cet appareil seulement.
sync-too-large-records = Le serveur conserve au plus 5 000 enregistrements par compte; le reste demeure sur cet appareil seulement.
sync-too-large-bytes = Le serveur conserve au plus 2 Mo de travail par compte; le reste demeure sur cet appareil seulement.
sync-too-large-other = Une partie de votre travail est trop volumineuse pour le serveur; elle reste sur cet appareil seulement.
