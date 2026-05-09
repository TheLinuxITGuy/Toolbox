# Modèle d'exécution et flux sudo

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│   Interface GUI │────▶│   main.sh    │────▶│ Package Manager │
│   (Rust/egui)   │     │   (Bash)     │     │ (apt/pacman/dnf)│
└────────┬────────┘     └──────┬───────┘     └────────┬────────┘
         │                     │                       │
         └─── mot de passe ────┴─── sudo -S ──────────┘
              (stdin pipe)
```

## Étapes d'exécution

1. L'utilisateur sélectionne des apps ou tâches admin dans l'UI
2. Il clique "Run Selected Tasks"
3. L'UI affiche un champ de mot de passe sudo
4. Au clic sur "Execute" :
   - Les commandes sont construites (`bash main.sh --label ... --package ... install`)
   - Chaque commande reçoit le mot de passe via `sudo -S`
   - Les tâches s'exécutent séquentiellement dans un thread dédié
   - La sortie stdout/stderr est affichée en temps réel dans le log

## Chemins et ressources

L'application localise ses ressources (`apps_config.csv`, `main.sh`, scripts admin) en remontant depuis l'exécutable ou via le répertoire de travail. Voir `src/main.rs → find_base_dir()`.

## Gestion du mot de passe

```rust
// src/main.rs — extrait simplifié
let mut child = Command::new("sudo")
    .arg("-S")                    // lire le mot de passe depuis stdin
    .arg("bash")
    .arg(script_path)
    .stdin(Stdio::piped())
    .spawn()?;

child.stdin.as_mut().unwrap()
    .write_all(password.as_bytes())?;
child.stdin.take();               // fermer stdin après envoi
```

Le mot de passe est conservé en mémoire dans un `String`, transmis au processus enfant, puis le `String` est libéré à la fin du scope.

## Construction des commandes

Toutes les commandes suivent un format fixe :

**Apps du catalogue :**
```
sudo -S bash <main.sh> --label "Firefox" --package "firefox" --flatpak "" --exec "firefox" install
```

**Tâches admin :**
```
sudo -S bash <update-system.sh>
```

Aucune commande n'est construite à partir de saisies utilisateur libres. Les paramètres viennent exclusivement de `apps_config.csv` (validé) ou de la liste statique des tâches admin.

## Communication inter-threads

```
┌─────────┐    mpsc::channel    ┌──────────────┐
│ Thread   │───────────────────▶│ Thread        │
│ worker   │   TaskUpdate enum  │ principal     │
│ (sudo)   │                    │ (GUI egui)    │
└─────────┘                    └──────────────┘

TaskUpdate:
  - Line(String)        → nouvelle ligne de sortie
  - TaskCompleted(usize) → tâche N terminée
  - AllDone             → toutes les tâches finies
  - Error(String)       → erreur fatale
```

## Risques connus et améliorations futures

- **Mot de passe en mémoire non protégé** : le `String` peut être swappé sur disque ou lu par un débogueur. Utiliser `secrets` crate avec `mlock` ou `madvise(MADV_DONTDUMP)`.
- **`sudo -S`** : transmet le mot de passe en clair via un pipe. `pkexec`/polkit offrirait une isolation plus forte (agent système, pas de mot de passe dans le processus applicatif).
- **Privilèges root complets** : les scripts s'exécutent en tant que root sans restriction. Un helper privilégié minimal (setuid ou daemon D-Bus) limiterait la surface d'attaque.
- **Séquentiel uniquement** : les tâches sont exécutées une par une. Pas de parallélisme. C'est un choix conservateur acceptable pour un outil desktop.
