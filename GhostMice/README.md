# GhostMice

Agent Windows 10/11 x64 natif, sans fenêtre, console, icône, configuration, réseau ou télémétrie. Rust stable, édition 2024. Un seul thread ; attente bloquante interruptible ; un événement `SendInput` relatif `(0, 0)` toutes les 30 secondes. Aucun clic, mouvement horizontal ou vertical intentionnel, repositionnement, verrouillage du pointeur ou maintien forcé de l'éveil.

La demande a été amendée : **les déplacements aléatoires et la vitesse de 5 pixels/s sont abandonnés** au profit de `SendInput(0, 0)`. `random.rs` conserve uniquement cette explication pour respecter l'arborescence demandée. Il n'y a donc ni PRNG ni distance à tirer. Zéro pixel signifie ici **envoyer un événement MOVE de delta nul**, et non ignorer le cycle.

## Étude de Move Mouse

Source examinée : [sw3103/movemouse](https://github.com/sw3103/movemouse), commit `0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188`. Les liens ci-dessous figent cette révision.

Constats confirmés par le code :

| Sujet | Constat et source |
| --- | --- |
| Stealth | Le convertisseur associe Stealth à `CursorDirection.None` : [convertisseur](https://github.com/sw3103/movemouse/blob/0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188/Move%20Mouse/Converters/CursorDirectionToStealthModeBoolConverter.cs). |
| Événement invisible | `None` appelle `MoveFromCurrentLocation(new Point())` : [action](https://github.com/sw3103/movemouse/blob/0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188/Move%20Mouse/Actions/MoveMouseCursorAction.cs). |
| Injection | Le wrapper construit un `INPUT_MOUSE` avec `MOUSEEVENTF_MOVE`, `dx/dy` relatifs, puis appelle `SendInput`. En Stealth, les deux deltas sont nuls. Le résultat numérique de `SendInput` n'est pas contrôlé : [wrapper](https://github.com/sw3103/movemouse/blob/0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188/Move%20Mouse/Wrappers/MouseCursorWrapper.cs). |
| Curseur visible | Les directions ordinaires envoient des pas relatifs de 1, avec `Thread.Sleep` entre les pas. Les trajets aller-retour tentent de revenir par déplacements opposés. Accélération, limites d'écran et utilisateur peuvent empêcher un retour exact. Le wrapper utilise `GetCursorPos` et peut interrompre si la position ne figure pas dans ses positions précédentes. Cette détection comporte des fenêtres de concurrence. |
| Autres API | Les déclarations incluent `GetLastInputInfo`, `GetCursorPos`, `SetCursorPos`, `mouse_event`, `keybd_event` et `SendInput` : [NativeMethods](https://github.com/sw3103/movemouse/blob/0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188/Move%20Mouse/Wrappers/NativeMethods.cs). L'action de position utilise `SetCursorPos` ; les actions clic et molette utilisent `mouse_event`. GhostMice ne reprend pas ces actions. |
| Intervalles | `Start()` choisit une durée fixe ou aléatoire et arme un `System.Threading.Timer` à un seul déclenchement. `PerformActions()` réappelle ensuite `Start()` : la durée des actions peut s'ajouter à l'intervalle. Options de pause, verrouillage et plages horaires : [view model](https://github.com/sw3103/movemouse/blob/0c53a9c8cd4c4e54a0abd2e73b45f61de11b7188/Move%20Mouse/ViewModels/MouseWindowViewModel.cs). |
| Inactivité | `PerformActions()` compare l'inactivité avant/après et journalise un avertissement si elle n'est pas réinitialisée. Le dépôt lui-même ne garantit donc pas cet effet. |

Hypothèses à tester : traitement d'un MOVE nul par chaque application, remise à zéro de l'inactivité dans un environnement donné, effet sur la présence Teams. Aucune de ces conclusions ne découle du seul appel d'API.

Move Mouse est sous GPL v3. GhostMice est une implémentation Rust nouvelle du mécanisme documenté, sans reprise de C#, d'images ni de marque. Par prudence de distribution, GhostMice est fourni sous **GPL-3.0-or-later**, avec le texte GPL dans `LICENSE`. Toute distribution du binaire doit être accompagnée de l'accès au source correspondant, au lockfile et aux instructions de compilation ; l'installateur seul n'est pas une offre de source suffisante. Les bibliothèques `windows-sys` et `windows-link` sont sous MIT OU Apache-2.0 ; leurs licences sont fournies dans `THIRD-PARTY-NOTICES.txt`.

## Mécanisme et limites

- `main.rs` : démarrage silencieux, instance unique et modes locaux de diagnostic/arrêt.
- `activity.rs` : injection et relevés de diagnostic.
- `scheduler.rs` : échéances monotones toutes les 30 s, premier événement après 30 s. La durée des relevés ne décale pas les échéances suivantes. Après suspension ou grand retard, pas de rafale de rattrapage. Windows n'est pas temps réel : une échéance peut être retardée.
- `windows.rs` : `SendInput`, mutex nommé, événement d'arrêt et lectures de diagnostic. Tous les handles possédés sont fermés.

Chaque cycle normal fournit exactement un `INPUT_MOUSE`, `dx=0`, `dy=0`, `mouseData=0`, `dwFlags=MOUSEEVENTF_MOVE`, `time=0` et une balise `dwExtraInfo=0x47484F53`. Il ne lit et ne restaure jamais la position en production : pas de course de repositionnement contre une souris physique. Aucun déplacement n'est demandé, y compris aux bords ou sur plusieurs écrans. Les applications peuvent néanmoins recevoir et interpréter l'événement synthétique.

Le retour `SendInput == 1` prouve seulement l'acceptation de l'événement par cette API. [Microsoft documente les restrictions UIPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput) : les programmes de niveau d'intégrité supérieur et les bureaux sécurisés peuvent bloquer l'injection. Un échec termine l'agent avec le code 1, sans boucle de relance ni élévation. Les erreurs de démarrage ont aussi le code 1 ; un doublon sort avec 0.

Quatre effets distincts :

1. **Maintenir Windows éveillé** : aucune utilisation de `SetThreadExecutionState` ; ni veille ni verrouillage ne sont garantis évités.
2. **Générer une entrée synthétique** : appel `SendInput` vérifié ; test par hook local disponible.
3. **Changer le compteur d'inactivité** : observable avec `GetLastInputInfo`, qui est [propre à la session](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlastinputinfo). Un mouvement physique concurrent rend l'attribution ambiguë.
4. **Présence Teams** : **non garantie et non testée ici**. Aucun accès au processus, aucune injection de code, aucun appel modifiant directement son statut. Un hook peut identifier l'entrée comme injectée : il ne s'agit pas d'une entrée matérielle indétectable.

Le mode invisible peut être ignoré par Windows ou une application. Aucun déplacement de secours n'est activé si cela arrive.

## Compilation

Pré-requis de développement uniquement : Rust stable avec Cargo, un linker Windows (Visual Studio Build Tools C++ + SDK pour MSVC, MinGW-w64 GCC pour GNU, ou LLVM-MinGW pour `x86_64-pc-windows-gnullvm`), Inno Setup **6** facultatif. Les outils ne sont pas intégrés à l'application et aucun runtime externe n'est installé chez l'utilisateur. `.cargo/config.toml` active la liaison statique des composants runtime pour MSVC et GNU-LLVM ; le binaire GNU-LLVM livré n'importe que des DLL Windows, dont l'UCRT fourni avec Windows 10/11. MSVC n'a pas été compilé dans cet environnement.

```powershell
cd GhostMice
cargo build --release
cargo test --locked
.\scripts\build.ps1
```

Profil Release : `opt-level="z"`, LTO, une unité de génération, panic abort et symboles retirés. Le script vérifie les codes de sortie et cherche `ISCC.exe` dans PATH puis les chemins habituels. Ajouter le répertoire d'un Inno portable à PATH si nécessaire. Il n'installe et ne télécharge rien. La première compilation Cargo nécessite les crates en cache ou un accès réseau ; l'exécution finale est entièrement locale.

Sorties : `target\release\ghostmice.exe` et, si Inno 6 est disponible, `dist\GhostMice-Setup-0.1.0.exe`. Le binaire est portable ; lancer seul le binaire ne configure pas le démarrage automatique.

## Installation et arrêt

Installation par compte dans `%LOCALAPPDATA%\Programs\GhostMice`, `PrivilegesRequired=lowest`, aucune UAC attendue. La page d'information indique explicitement le démarrage automatique et l'absence de garantie Teams. L'installateur crée `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\GhostMice` contenant le chemin entre guillemets. Il démarre l'agent à la fin. La désinstallation signale l'événement d'arrêt, attend la libération du mutex, puis supprime fichiers et valeur Run. Si l'arrêt échoue, elle s'annule avec une erreur plutôt que de prétendre avoir arrêté l'agent.

Le mutex et l'événement utilisent l'espace de noms Windows `Global` avec le SID réel du compte dans leur nom. Un second lancement du même compte, même depuis un autre chemin ou une autre session, sort immédiatement. L'arrêt coopératif peut ainsi atteindre l'agent de ce compte dans une autre session. Aucun service ni privilège administrateur n'est nécessaire pour ces objets ; aucune terminaison forcée par nom de processus. La présence multi-session est prévue par le code mais doit encore être testée avec deux sessions réelles.

Pour arrêter manuellement (attente maximale de 5 s après signalement) :

```powershell
Start-Process .\target\release\ghostmice.exe -ArgumentList '--stop' -WindowStyle Hidden -Wait
```

## Validation locale reproductible

Fermer toute instance avant les tests. Les tests ignorés injectent un événement réel et attendent réellement 30 s :

```powershell
cargo test --locked -- --test-threads=1
cargo test --locked -- --ignored --nocapture --test-threads=1
Start-Process .\target\release\ghostmice.exe -ArgumentList '--test' -WindowStyle Hidden -Wait
Get-Content "$env:TEMP\GhostMice-test.log"
```

`--test` garde les mêmes 30 s, exécute deux cycles puis sort (~60 s). Seul ce mode écrit `%TEMP%\GhostMice-test.log` (écrasé au prochain test), avec temps monotone, deltas, nombre accepté, position avant/après et ticks de dernière entrée. Le journal reste local et n'est pas une configuration. En présence d'une instance normale, le test sort comme doublon sans réécrire le journal : arrêter l'instance d'abord. Une sonde par hook `WH_MOUSE_LL` compte les événements portant la balise et le flag `LLMHF_INJECTED`, sans bloquer ni modifier l'événement, puis retire le hook. Un compte de zéro est rapporté comme tel : ce n'est pas un test de réception réussi. Une position avant/après identique seule ne prouverait pas l'absence de déplacement transitoire ; ici le contenu nul de l'entrée est aussi vérifié.

Le script `scripts\verify-install.ps1` fait une installation réelle temporaire, vérifie Run, lancement, doublon, arrêt, relance via la commande Run, puis désinstallation et absence de processus de ce chemin. Il refuse une installation GhostMice préexistante. Il ne remplace pas une véritable fermeture/réouverture de session.

Essais interactifs restant nécessaires :

- Lancer `--test` en déplaçant physiquement la souris pendant les deux cycles ; vérifier l'absence de saut ou retour forcé. Le curseur peut naturellement différer avant/après. Essayer les bords, plusieurs écrans et RDP si utilisés.
- Installer, fermer puis rouvrir réellement la session, vérifier qu'une seule instance fonctionne, puis désinstaller. Ce test interrompt la session et n'est pas automatisé ici.
- Teams : relever version, compte/politique d'entreprise, session locale/RDP, veille et statut initial. Observer sans GhostMice au-delà du délai habituel d'inactivité, puis répéter avec GhostMice sans toucher souris/clavier, avec Teams au premier plan puis en arrière-plan. Comparer depuis un autre client/observateur si possible. Noter heure de chaque changement et répéter. Un relevé `GetLastInputInfo` ou le statut local seul ne suffit pas à conclure. Résultat inconnu tant que cette expérience n'est pas réalisée.

Voir `TEST-RESULTS.md` pour les vérifications réellement exécutées, les versions des outils et leurs limites.
