# Security Policy

## Permissions requises

L'application a besoin des privilèges suivants pour fonctionner :

| Permission | Pourquoi | Portée | Code concerné |
|-----------|----------|--------|---------------|
| `sudo` / root | Installer/désinstaller des paquets natifs et Flatpak, exécuter des scripts admin | Temporaire, par tâche | `src/main.rs` → `run_selected()`, `main.sh` |
| Accès réseau | Télécharger les paquets depuis les dépôts (apt, pacman, dnf, flatpak) | Pendant l'installation | Appels package manager dans `main.sh` |
| Lecture fichiers locaux | Lire `apps_config.csv`, les scripts shell, les assets | Répertoire de base de l'app uniquement | `src/main.rs` → `load_apps()`, `find_base_dir()` |
| Écriture fichiers système | Modifier les paquets installés via le gestionnaire de paquets | Via le gestionnaire de paquets | `main.sh` → `install_native()`, `remove_native()` |

## Flux d'élévation de privilèges

```
┌──────────────┐     ┌───────────────┐     ┌──────────────────┐
│  GUI (egui)  │────▶│  Thread dédié │────▶│  sudo -S bash    │
│  password    │     │  stdin pipe   │     │  main.sh ...     │
│  TextEdit    │     │               │     │                  │
└──────────────┘     └───────────────┘     └────────┬─────────┘
                                                    │
                                          ┌─────────▼─────────┐
                                          │ Package Manager   │
                                          │ (apt/pacman/dnf)  │
                                          └───────────────────┘
```

### Détail du flux

1. L'utilisateur sélectionne des tâches et clique **Run Selected Tasks**
2. Une fenêtre modale demande le mot de passe sudo
3. Le mot de passe est stocké dans un `String` en mémoire (jamais écrit sur disque)
4. Un thread dédié lance les commandes une par une :
   ```rust
   let mut child = Command::new("sudo")
       .arg("-S")                         // lire mot de passe depuis stdin
       .arg("bash")
       .arg(script_path)
       .arg("--label").arg(&entry.label)
       .arg("--package").arg(&entry.package_name)
       // ...
       .stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .spawn()?;

   // Transmission du mot de passe via le pipe stdin
   child.stdin.as_mut().unwrap()
       .write_all(password.as_bytes())?;
   child.stdin.take();                    // fermer stdin après envoi
   ```
5. La sortie stdout/stderr est affichée en temps réel dans la zone de log
6. Le `String` contenant le mot de passe est libéré automatiquement à la fin du scope

## Risques connus

| Risque | Sévérité | Statut |
|--------|----------|--------|
| Mot de passe en clair dans un `String` Rust non protégé | Medium | Accepté temporairement — migration vers `secrets` crate ou polkit prévue |
| `sudo -S` moins sûr que polkit/pkexec | Medium | Migration recommandée dans une PR future |
| Exécution de scripts avec privilèges root complets | Medium | Helper privilégié minimal à envisager |
| Fallback `current_dir()` dans `find_base_dir()` | High | **Corrigé** dans PR #6 |
| Pas de validation des noms de paquets | High | **Corrigé** dans PR #6 |

## Signaler une vulnérabilité

Si vous découvrez un problème de sécurité :

1. **Ne créez pas d'issue publique** si la vulnérabilité est sensible
2. Contactez l'auteur directement
3. Pour les problèmes non sensibles, ouvrez une issue avec le tag `security`

## Versions supportées

| Version | Support |
|---------|---------|
| main (dernier commit) | ✅ Support actif |
| Releases taggées | ✅ Corrections de sécurité backportées au cas par cas |
