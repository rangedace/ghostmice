# Vérifications du 19 octobre 2026

Environnement : Windows x64, compte **non administrateur** (`IsInRole(Administrator)=False`). Rust stable 1.99.1, édition 2024, hôte `x86_64-pc-windows-gnullvm`, LLVM-MinGW 20261006, Inno Setup **6.7.3**. Outils de compilation placés dans le répertoire temporaire, sans modification permanente du PATH. Leurs prérequis ne font pas partie du produit.

## Résultats exécutés

| Exigence | Résultat observé |
| --- | --- |
| Compilation Release | `cargo build --release` réussi ; `scripts/build.ps1` compile avec `--locked` et génère l'installateur. |
| Vérification statique | `cargo fmt --check` et `cargo clippy --all-targets --locked -- -D warnings` réussis après correction d'un avertissement Clippy. |
| Tests unitaires | 3 réussis : échéances, contenu exact de l'entrée, exclusion/réacquisition du mutex. 2 tests natifs ignorés par défaut, exécutés séparément ci-dessous. |
| Distance / boutons | Test du `INPUT` : relatif `(0,0)`, `mouseData=0`, seul `MOUSEEVENTF_MOVE`. Les distances aléatoires ont été retirées à la demande de l'utilisateur. |
| Attente réelle | Test `real_interval` : **30 009 ms**, sans attente active. |
| Diagnostic Release | Deux cycles à **30 005 ms** et **60 005 ms**, soit **30 000 ms** entre les deux. Code de sortie 0. |
| Curseur | Pour les deux cycles Release : avant/après `(1232,1059)`, inchangé. Aucun retour n'est nécessaire, aucune API de repositionnement n'est appelée. |
| Injection / hook | `SendInput` accepte 1 événement. La sonde `WH_MOUSE_LL` rapporte **0 événement portant la balise**. La réception par hook n'est donc pas établie. |
| Inactivité | Voir relevés bruts ci-dessous. Le tick change, mais une activité extérieure concurrente ne peut pas être exclue ; pas de garantie générale de remise à zéro. |
| Instance unique | Test unitaire réussi, puis lancement réel d'un doublon pendant le test d'installation : le doublon sort avec 0 et il reste une seule instance. |
| Installation | Installation silencieuse réelle, sans élévation, dans `%LOCALAPPDATA%\GhostMice-Installation-Test`. Fichier présent, commande HKCU Run exacte et entre guillemets, une instance lancée à la fin. |
| Démarrage automatique | Entrée HKCU Run vérifiée et commande de démarrage exécutée avec succès après arrêt. **Réouverture réelle de session non effectuée**. |
| Désinstallation | Réelle, avec agent actif : code 0, absence de l'exécutable, de la valeur Run, de l'entrée de désinstallation et des processus de cette installation. Le test nettoie l'installation temporaire. |
| Portabilité locale | Binaire Release lancé avec PATH réduit à `System32` et Windows ; diagnostic complet réussi. Inspection PE : x64, sous-système Windows (sans console), imports de DLL Windows uniquement ; aucune DLL de runtime propre à LLVM/MinGW. |
| Repos | Sur un échantillon de **10 secondes**, diagnostic en attente : **0 ms de CPU** mesuré, **5 689 344 octets** de mémoire résidente. Observation locale, pas une garantie de mesure sur toute machine. |

Relevés du diagnostic Release :

```text
cycle_ms=30005 sent=1 dx=0 dy=0 before=(1232, 1059) after=(1232, 1059) unchanged=true last_input_before=5663906 last_input_after=5691765
cycle_ms=60005 sent=1 dx=0 dy=0 before=(1232, 1059) after=(1232, 1059) unchanged=true last_input_before=5693921 last_input_after=5721765
```

## Échecs et corrections pendant la réalisation

- Le premier environnement GNU ne disposait pas de `dlltool`, puis le mélange GCC/LLVM ne fournissait pas `libgcc`. La compilation finale utilise l'hôte GNU-LLVM avec LLVM-MinGW correspondant.
- Le premier binaire importait `libunwind.dll`. `.cargo/config.toml` active désormais la liaison statique de ces composants pour GNU-LLVM ; les imports ont été revérifiés et le binaire exécuté sans outils dans PATH.
- Le premier test de hook supposait à tort qu'un MOVE nul serait livré au hook. Il a **échoué** avec `hook_events=0`. La sonde rapporte maintenant ce résultat expérimental sans assertion de livraison ; elle contrôle seulement l'acceptation par l'API et la position stationnaire. Le résultat nul reste explicitement visible.
- La première tentative d'installation automatisée attendait l'arbre de processus avec `Start-Process -Wait` et restait en attente de l'agent. L'essai a été arrêté et nettoyé. Le script attend maintenant le seul processus parent avec `WaitForExit()` ; le cycle complet a ensuite réussi. Un diagnostic lancé pendant cet essai a été rejeté comme doublon, puis relancé seul pour obtenir les relevés ci-dessus.

## Non vérifié

- **Microsoft Teams** : aucun essai de présence effectué ; reconnaissance inconnue. Le protocole manuel est dans le README.
- Mouvements **physiques** de souris pendant un cycle, bords multi-écrans et RDP : essais interactifs non réalisés. L'absence de repositionnement et le delta nul évitent par construction de ramener le curseur à une ancienne position, mais ce raisonnement ne remplace pas l'essai humain.
- Vraie fermeture/réouverture de session pour le démarrage automatique.
- Deux sessions Windows simultanées : le mutex et l'arrêt utilisent `Global` avec SID du compte, mais ce scénario n'a pas été exécuté ici.
- Compilation MSVC, Windows ARM64 et autres machines : non exécutées.

Les essais ne démontrent ni le maintien de l'éveil, ni une activité matérielle, ni un statut Teams. Le produit ne modifie pas Teams et ne remplace pas le delta nul par un mouvement visible en cas d'inefficacité.

## Vérification après renommage en GhostMice

Le 19 octobre 2026, après renommage du projet, du paquet Cargo, du binaire, de l'installateur, des objets de synchronisation Windows, de l'entrée Run et des scripts :

- `cargo fmt --check` et `cargo clippy --all-targets --locked -- -D warnings` réussis.
- `cargo test --locked` : 3 tests réussis, 2 tests natifs ignorés par défaut.
- `scripts/build.ps1` : binaire `ghostmice.exe` et installateur `GhostMice-Setup-0.1.0.exe` générés avec succès.
- `scripts/verify-install.ps1` : installation temporaire, entrée Run GhostMice, instance unique, arrêt, relance et désinstallation vérifiés avec succès, sans agent ni entrée Run laissés par le test.

Les mesures détaillées plus haut proviennent de la validation initiale ; les deux tests natifs et le diagnostic de 60 secondes n'ont pas été réexécutés pour le renommage.
