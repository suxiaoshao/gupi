app-title = Gupi
menu-settings = Réglages
menu-show-main = Fenêtre principale
menu-quit = Quitter Gupi
startup-welcome = Bienvenue dans Gupi
temporary-setup-required = Terminez la configuration dans Réglages pour démarrer une conversation temporaire.
startup-checking = Vérification…
startup-quitting = Finalisation des opérations…
settings-pi-command = Exécutable Pi
settings-path-help = Laissez vide pour rechercher pi automatiquement, ou indiquez un chemin absolu vers l’exécutable sans arguments.
settings-theme = Apparence
settings-language = Langue
settings-save-pi = Enregistrer le chemin de Pi
settings-reload = Recharger
settings-write-current = Écrire les réglages appliqués sur le disque (conserver le brouillon)
theme-system = Système
theme-light = Clair
theme-dark = Sombre
language-system = Système
language-english = English
language-chinese = 简体中文
recovery-config-title = La configuration nécessite votre attention
recovery-startup-title = Gupi n’a pas pu démarrer
error-instance-startup = Vérifiez les autorisations du dossier de configuration ou fermez les autres copies de Gupi en cours d’exécution, puis réessayez.
recovery-pi-title = Configurer Pi
recovery-confirm = Continuer ? Le rechargement n’abandonne le brouillon qu’en cas de réussite. Une sauvegarde est créée avant la réinitialisation.
recovery-backup = Sauvegarde
error-config-read = Impossible de lire la configuration. Vérifiez son emplacement et ses autorisations, puis rechargez-la.
error-config-parse = La configuration est invalide. Rechargez un fichier corrigé ou sauvegardez puis réinitialisez le fichier actuel.
error-config-validation = Saisissez un nom d’exécutable ou un chemin absolu vers l’exécutable, sans arguments.
error-config-write = Échec de l’enregistrement. Le brouillon et les réglages appliqués sont conservés. Vérifiez le fichier de configuration puis réessayez.
error-pi-probe = Impossible de vérifier Pi. Vérifiez le chemin de l’exécutable et son environnement Node, puis réessayez.
action-check-pi = Vérifier Pi à nouveau
action-reset = Sauvegarder et réinitialiser
action-confirm = Confirmer
action-cancel = Annuler
home-ready = Pi est disponible
home-description = Cette version propose la configuration de l’environnement et les préférences du bureau. La prise en charge des conversations est prévue pour la prochaine étape.
action-locate = Afficher le dossier de configuration
error-pi-timeout = Pi n’a pas terminé la vérification de version en 15 secondes.
error-pi-output = Pi a renvoyé plus de sortie que la vérification de version ne l’autorise.
error-pi-version = Pi s’est arrêté avec une erreur ou n’a pas renvoyé de version valide.
error-log = La journalisation dans un fichier est indisponible. Vérifiez le dossier des journaux et ses autorisations.

setup-tagline = Votre espace de bureau natif pour Pi
setup-intro = Une configuration rapide pour commencer.
setup-start = Commencer
setup-language-title = Installez-vous confortablement
setup-search-language = Rechercher une langue…
setup-system-language = Langue du système : { $language }
setup-appearance-title = Personnalisez votre espace
setup-color-mode = Mode de couleur
setup-system-accent = Couleur d’accentuation du système
setup-pi-title = Connecter Pi
setup-pi-step = Configuration de Pi
setup-pi-help = Vérifiez une installation locale de Pi maintenant, ou ignorez cette page pour la configurer plus tard.
setup-check-pi = Vérifier Pi
setup-back = Retour
setup-next = Continuer
setup-finish = Terminer la configuration
setup-saving = Enregistrement…
light-themes = Thèmes clairs
dark-themes = Thèmes sombres

# Conversation workspace
conversation-new = Nouvelle conversation
conversation-sidebar = Barre latérale des conversations
conversation-history = Historique des conversations
conversation-restoring = Restauration des brouillons…
conversation-discovering = Recherche des conversations… { $count } fichiers
conversation-read-progress = Lecture de { $completed } / { $total } fichiers
conversation-refresh-progress = Actualisation de { $completed } / { $total } fichiers
conversation-project = Choisir le dossier de travail
conversation-project-search-placeholder = Rechercher des projets
conversation-project-search-empty = Aucun projet correspondant
conversation-open-folder = Ouvrir un dossier…
conversation-untitled = Conversation sans titre
conversation-idle = Inactive
conversation-loading = Ouverture de la session
conversation-running = En cours
conversation-failed = Erreur nécessitant votre attention
conversation-waiting = En attente de saisie
conversation-search = Rechercher des conversations
conversation-refresh = Actualiser les sessions
conversation-refresh-current = Actualiser la session actuelle
conversation-actions = Actions de session
conversation-show-sidebar = Afficher la barre latérale des sessions
conversation-hide-sidebar = Masquer la barre latérale des sessions
conversation-scan-failed = Impossible de charger le catalogue des conversations
conversation-search-placeholder = Rechercher des noms, messages ou chemins de projet
conversation-search-empty = Aucune conversation correspondante
conversation-rename = Renommer…
conversation-delete = Déplacer vers la corbeille
conversation-delete-failed = Impossible de supprimer la conversation : { $error }
conversation-copy-path = Copier le chemin
conversation-clone = Dupliquer la conversation
conversation-export = Exporter la conversation au format HTML…
conversation-exported = Conversation exportée vers { $path }
conversation-stop = Arrêter la génération
conversation-close-run = Fermer le processus
conversation-show-less = Afficher moins
conversation-show-more = Afficher plus…
conversation-continue = Continuer à partir d’ici
conversation-fork = Créer une branche à partir d’ici
conversation-preview = Aperçu d’une autre branche
conversation-return-current = Revenir à la branche actuelle
conversation-welcome = Démarrer une conversation
conversation-welcome-project = Que souhaitez-vous faire dans { $project } ?
conversation-empty-history = Cette conversation ne contient encore aucun message.
conversation-bottom = Aller en bas
conversation-copy = Copier
conversation-compaction = Résumé de la compaction du contexte
conversation-branch-summary = Résumé de la branche
conversation-working = En cours…
conversation-process = Afficher le processus
conversation-details = Afficher ou masquer les détails
tool-detail-offset = Ligne de départ :
tool-detail-line-limit = Nombre de lignes demandé :
tool-detail-timeout = Délai d’expiration (secondes) :
tool-detail-output = Sortie
tool-detail-input = Entrée
tool-detail-additional = Informations supplémentaires
tool-detail-error = Erreur
tool-detail-glob = Motif de fichiers :
tool-detail-ignore-case = Ignorer la casse :
tool-detail-literal = Recherche littérale :
tool-detail-context = Lignes de contexte :
tool-detail-result-limit = Nombre de résultats demandé :
tool-detail-old-text = Texte d’origine demandé
tool-detail-new-text = Texte de remplacement demandé
tool-detail-truncated = Pi a tronqué cette sortie ; le contenu affiché est incomplet.
tool-detail-lines-truncated = Pi a raccourci certaines lignes de résultat.
tool-detail-match-limit = Limite de correspondances atteinte :
tool-detail-results-limited = Limite de résultats atteinte :
tool-detail-entries-limited = Limite d’entrées du répertoire atteinte :
tool-detail-full-output = Fichier de sortie complet :
tool-detail-image-unavailable = Impossible d’afficher cette image.
conversation-role-user = Message de l’utilisateur
conversation-role-assistant = Message de l’assistant
conversation-role-tool = Résultat de l’outil
conversation-event = Événement de session
conversation-close-history = Fermer l’historique des conversations
conversation-source = Afficher le fichier source de la session
conversation-reconnect = Reconnecter
conversation-save-error = Impossible d’enregistrer le brouillon
conversation-interrupted = Exécution arrêtée ; le contenu existant est conservé.
conversation-no = Non
conversation-submit = Envoyer
conversation-input = Écrire un message
conversation-model = Modèle
conversation-load-options = Connectez-vous à Pi pour charger les modèles
conversation-thinking = Niveau de réflexion
conversation-unknown = Indisponible
conversation-context = Contexte de branche de l’exécution actuelle
conversation-auto-compaction = Compaction automatique
conversation-on = Activé
conversation-off = Désactivé
conversation-tokens = Total entrées / sorties
conversation-cache = Total lecture / écriture du cache
conversation-cache-hit = Taux de réussite récent du cache
conversation-cost = Coût indiqué par Pi
conversation-statistics = Statistiques d’utilisation
conversation-send = Envoyer (Entrée) ; Alt+Entrée met une tâche de suivi en file d’attente
conversation-sending = Envoi…
conversation-graph-current = Actuelle
conversation-graph-preview = Aperçu
conversation-graph-left = Afficher les colonnes à gauche (ou faire défiler horizontalement)
conversation-graph-right = Afficher les colonnes à droite (ou faire défiler horizontalement)
conversation-graph-reveal = Afficher la sélection
conversation-graph-column = Graphe
conversation-graph-message = Message
conversation-catalog-empty = Aucune conversation pour le moment

conversation-thinking-content = Réflexion
conversation-thinking-running = Réflexion…
conversation-tool-group = Appels d’outils ({ $count })
conversation-tool-group-read = Fichiers lus ({ $count })
conversation-tool-group-bash = Commandes ({ $count })
conversation-tool-group-search = Recherches ({ $count })
conversation-tool-group-edit = Modifications de fichiers ({ $count })
conversation-tool-line = { $action } { $summary }
conversation-tool-action-read =
    { $state ->
        [running] Lecture en cours
        [complete] Lu
        [failed] Échec de la lecture
       *[unfinished] Lecture inachevée
    }
conversation-tool-action-write =
    { $state ->
        [running] Écriture en cours
        [complete] Écrit
        [failed] Échec de l’écriture
       *[unfinished] Écriture inachevée
    }
conversation-tool-action-edit =
    { $state ->
        [running] Modification en cours
        [complete] Modifié
        [failed] Échec de la modification
       *[unfinished] Modification inachevée
    }
conversation-tool-action-bash =
    { $state ->
        [running] Exécution en cours
        [complete] Exécuté
        [failed] Échec de la commande
       *[unfinished] Exécution inachevée
    }
conversation-tool-action-search =
    { $state ->
        [running] Recherche en cours
        [complete] Recherché
        [failed] Échec de la recherche
       *[unfinished] Recherche inachevée
    }
conversation-tool-action-other =
    { $state ->
        [running] Appel de { $name } en cours
        [complete] { $name } appelé
        [failed] Échec de l’appel de { $name }
       *[unfinished] Appel de { $name } inachevé
    }

conversation-processed = Durée de traitement : { $duration }
conversation-processed-failed = Échec après { $duration }
conversation-processed-stopped = Arrêt après { $duration }
conversation-copied = Copié
conversation-copy-failed = Échec de la copie. Veuillez réessayer.
conversation-usage-title = Utilisation de la requête
conversation-usage-model = Modèle
conversation-usage-provider = Fournisseur
conversation-usage-input = Jetons d’entrée
conversation-usage-output = Jetons de sortie
conversation-usage-cache-read = Jetons lus depuis le cache
conversation-usage-cache-write = Jetons écrits dans le cache
conversation-usage-total = Total des jetons
conversation-usage-cost = Coût

composer-context-used = Jetons utilisés
composer-context-limit = Capacité du contexte
composer-context-percent = Contexte utilisé
composer-token-input = Total des jetons d’entrée
composer-token-output = Total des jetons de sortie
composer-token-cache-read = Total des jetons lus depuis le cache
composer-token-cache-write = Total des jetons écrits dans le cache
composer-token-usage = Utilisation des jetons de la session

conversation-model-search = Rechercher des modèles…
conversation-model-empty = Aucun modèle correspondant
conversation-thinking-off = Désactivé
conversation-thinking-minimal = Minimal
conversation-thinking-low = Faible
conversation-thinking-medium = Moyen
conversation-thinking-high = Élevé
conversation-thinking-xhigh = Très élevé
conversation-thinking-max = Maximum

conversation-model-reasoning = Raisonnement
conversation-model-vision = Vision

conversation-history-reply = Réponse de l’assistant
conversation-history-brief = Bref : messages de l’utilisateur et réponses finales
conversation-history-detailed = Détaillé : messages, outils et résumés
conversation-history-progress = Progression de l’assistant
conversation-history-thinking = Réflexion
conversation-history-calls = Appels d’outils
conversation-history-failed = Échec de l’exécution
conversation-history-stopped = Interrompu
conversation-history-empty = Réponse sans texte
composer-model-thinking = Modèle et niveau de réflexion
composer-model-refresh = Recharger les modèles et niveaux de réflexion
composer-thinking-unavailable = Non pris en charge
composer-model-loading = Chargement des modèles…
composer-thinking-loading = Chargement des niveaux de réflexion…
composer-model-confirming = Confirmation des réglages du modèle…
composer-stats-loading = Chargement de l’utilisation…
composer-stats-retry = Réessayer de charger l’utilisation
conversation-checking-file = Vérification du fichier de conversation…
conversation-connecting = Connexion à Pi…
conversation-history-loading = Chargement de l’historique de la conversation…
conversation-history-refreshing = Actualisation de l’historique de la conversation…
conversation-fork-options-loading = Chargement des options de branche…
conversation-fork-options-retry = Réessayer de charger les options de branche
history-canvas-zoom-in = Zoom avant
history-canvas-zoom-out = Zoom arrière
history-canvas-fit = Ajuster l’arbre (0)
history-canvas-current = Localiser le nœud d’exécution
history-canvas-expand = Développer { $count } nœuds
history-canvas-collapse = Réduire { $count } nœuds de processus en conservant les marqueurs de navigation
history-canvas-empty = Aucun nœud d’historique à afficher
history-canvas-help = Arbre de conversation : faites défiler ou glissez pour déplacer, pincez ou utilisez plus/moins pour zoomer, les flèches pour sélectionner, Entrée pour prévisualiser et E pour développer le segment suivant

history-view-tree = Arbre
history-view-list = Liste
history-level-brief = Bref
history-level-detailed = Détaillé
history-level-all = Tout
history-level-all-description = Tout : y compris les réglages, étiquettes et entrées personnalisées
history-scope-all = Toutes les branches
history-scope-branch = Cette branche uniquement
history-model-change = Changement de modèle
history-thinking-change = Changement du niveau de réflexion
history-session-info = Modification des informations de session
history-label-change = Modification de l’étiquette
history-custom-record = Entrée personnalisée


history-content = Contenu
history-range = Portée

command-palette = Palette de commandes
command-focus-input = Placer le focus dans la zone de saisie
command-model = Choisir le modèle et le niveau de réflexion…
command-copy-last-answer = Copier la dernière réponse
command-compact = Compacter le contexte
conversation-compacting = Compaction du contexte…
command-history-description = Afficher l’historique ou choisir un message utilisateur pour créer une branche
command-show-history = Afficher l’historique
command-hide-history = Masquer l’historique
command-current-session = Session actuelle :
command-unavailable = Indisponible dans l’état actuel
command-target-changed = Cette conversation n’est plus disponible. Sélectionnez-en une autre.
command-scanning = Chargement des conversations…
command-loading = Chargement des commandes Pi…
command-no-matches = Aucune commande Pi correspondante
conversation-reconnect-unconfirmed = La fin du processus Pi n’a pas pu être confirmée. Redémarrez Gupi avant de vous reconnecter.
action-retry = Réessayer

command-search-placeholder = Rechercher des actions ou saisir une commande
command-empty = Aucune commande Pi n’est disponible dans cette session

command-group-app = Application
command-group-extensions = Commandes d’extension
command-group-skills = Compétences
command-group-prompts = Modèles d’instructions
command-scope-user = Personnel
command-scope-project = Projet
command-scope-temporary = Temporaire
command-connection-unavailable = Pi n’est pas prêt
command-dismiss = Fermer
command-complete = Terminer
command-send-text = Envoyer
command-execute = Exécuter

command-scope-current = Session actuelle

conversation-working-duration = En cours depuis { $duration }
conversation-tool-group-skill = Compétences lues ({ $count })
conversation-shell-line = { $shell } · { $action } { $summary }
conversation-tool-action-skill =
    { $state ->
        [running] Lecture de la compétence en cours
        [complete] Compétence lue
        [failed] Échec de la lecture de la compétence
       *[unfinished] Lecture de la compétence inachevée
    }

# Unified settings
settings-page-general = Général
settings-page-pi = Pi
settings-page-keys = Raccourcis clavier
settings-page-skills = Compétences
settings-page-prompts = Instructions
settings-page-about = À propos
settings-key-reset = Rétablir la valeur par défaut
settings-key-reset-all = Rétablir toutes les valeurs par défaut
settings-key-reset-all-confirm = Rétablir les raccourcis par défaut ? Tous les raccourcis personnalisés seront supprimés.
settings-key-clear = Effacer le raccourci
settings-key-cancel = Annuler les modifications
settings-key-unbound = Aucun raccourci attribué
settings-key-recording = Saisissez un raccourci, Échap pour annuler
settings-key-conflict = Raccourci en conflit ou invalide
settings-key-invalid = Saisissez un raccourci valide ou laissez vide pour supprimer l’attribution.
settings-resource-refresh = Recharger
settings-resource-reload-help = Ressources personnelles uniquement. Rechargez manuellement les sessions existantes pour appliquer les modifications.
settings-resource-working = Opération en cours…
settings-resource-pi-required = Enregistrez et vérifiez d’abord un exécutable Pi fonctionnel dans les réglages de Pi.
settings-resource-invalid-name = Utilisez un nom de fichier valide sans séparateurs de chemin.
settings-resource-register = Ajouter localement
settings-resource-create = Créer
settings-resource-empty = Aucune ressource correspondante.
settings-resource-delete = Déplacer vers la corbeille
settings-resource-save = Enregistrer le texte
shortcut-save-task = Enregistrer
settings-resource-loading = Lecture du texte…
settings-resource-confirm-remove = Seul le fichier sélectionné est déplacé dans la corbeille ; son dossier est conservé.
settings-package-install = Installer le paquet
settings-package-update = Mettre à jour
settings-package-remove = Supprimer
settings-package-source-help = Source du paquet : npm:name, URL Git ou chemin local absolu.
settings-package-extensions = Extensions { $count }
settings-package-skills = Compétences { $count }
settings-package-prompts = Modèles { $count }
settings-package-themes = Thèmes { $count }
settings-skill-search = Rechercher des compétences par nom, description, chemin ou source
settings-skill-collapse = Réduire
settings-resource-edit = Modifier…
settings-about-gupi = Version
settings-about-pi = Version
settings-about-path = Chemin résolu
settings-about-status = État de la vérification
settings-about-unavailable = Pas encore disponible
settings-editor-discard = Abandonner les modifications de texte non enregistrées ?
settings-package-confirm-remove = Retire le paquet de la portée Pi sélectionnée. Les dossiers sources locaux sont conservés.
settings-package-heading = Paquets installés
settings-extension-heading = Extensions autonomes
settings-template-heading = Modèles de commandes
settings-system-heading = Instructions système
settings-config-heading = Fichier de configuration
settings-config-open = Ouvrir le fichier de configuration
settings-mode-help = Suivre le système pour basculer automatiquement entre les thèmes clair et sombre.
settings-light-help = Utilisé lorsque le mode clair est actif.
settings-dark-help = Utilisé lorsque le mode sombre est actif.
settings-pi-unsaved = Chemin non enregistré. La vérification valide la saisie actuelle sans l’enregistrer.
settings-pi-saved = Le chemin actuel est enregistré. Vérifiez à nouveau la disponibilité de Pi.
settings-resource-location = Afficher l’emplacement
settings-package-source = Source du paquet
settings-package-empty = Aucun paquet Pi personnel n’est installé.
settings-package-resources-empty = Aucune ressource déclarée trouvée dans ce paquet.
settings-resource-name = Nom
settings-resource-none = Aucune ressource personnelle de ce type pour le moment.
settings-resource-view = Afficher
settings-resource-readonly-badge = Lecture seule
settings-source-personal = Personnel
settings-source-external = Externe
settings-system-replace = Remplacer l’instruction système par défaut
settings-system-replace-help = Remplace entièrement l’instruction système par défaut de Pi.
settings-system-append = Ajouter des instructions personnelles
settings-system-append-help = Conserve l’instruction système par défaut et ajoute ces instructions.
settings-resource-success = Opération terminée pour « { $target } ». Rechargez manuellement les sessions existantes.
settings-resource-failed = Échec de l’opération pour « { $target } ».
settings-about-help = Affiche le dernier contrôle du chemin enregistré. Ouvrez les réglages de Pi pour le vérifier à nouveau.
settings-key-reveal-session = Afficher le fichier de session

settings-key-group-app = Application
settings-key-group-conversation = Conversation
settings-key-group-files = Fichiers de session

settings-package-kind-extensions = Extensions
settings-package-kind-skills = Compétences
settings-package-kind-themes = Thèmes
settings-package-kind-prompts = Modèles
settings-package-expand = Développer le contenu du paquet
settings-package-collapse = Réduire le contenu du paquet

settings-template-search = Rechercher des noms, descriptions ou sources de modèles
settings-template-collapse = Réduire l’aperçu

temporary-title = Conversations temporaires
attachment-add = Joindre des fichiers
attachment-remove = Supprimer la pièce jointe
attachment-file = Fichier
attachment-clipboard = Image du presse-papiers
image-preview-close = Fermer l’aperçu
image-preview-open = Ouvrir l’aperçu de l’image
image-preview-zoom-in = Agrandir l’image
image-preview-zoom-out = Réduire l’image
shortcut-global = Raccourcis globaux
shortcut-launcher = Fenêtre temporaire
shortcut-add = Ajouter une tâche modèle…
shortcut-edit = Modifier la tâche modèle…
shortcut-delete = Supprimer la tâche modèle
shortcut-name = Nom
shortcut-template = Modèle de commande
shortcut-source = Source de saisie
shortcut-selection = Texte sélectionné
shortcut-clipboard = Presse-papiers
shortcut-fallback = Texte sélectionné, sinon presse-papiers
shortcut-model = Modèle
shortcut-thinking = Niveau de réflexion
shortcut-default-model = Modèle par défaut de Pi
shortcut-default-thinking = Niveau de réflexion par défaut de Pi
shortcut-personal = Personnel
shortcut-template-required = Sélectionnez un modèle de commande.
shortcut-reload-options = Recharger les options
temporary-clean-released = Espaces de travail libérés
temporary-clean = Nettoyer
temporary-clean-help = Déplacer les espaces de travail temporaires libérés vers la corbeille. Les conversations actives sont conservées.
temporary-cleaned = Espaces de travail déplacés vers la corbeille
shortcut-tasks = Tâches modèles

shortcut-select-template = Choisir un modèle d’instructions

temporary-search-placeholder = Rechercher des conversations temporaires
temporary-search-empty = Aucune conversation temporaire correspondante
temporary-toggle-input = Changer de mode de saisie
temporary-hide = Masquer
session-search-open = Ouvrir la conversation

temporary-actions = Actions
temporary-paste-answer = Coller la dernière réponse
temporary-reveal-workspace = Afficher le dossier de travail
temporary-search-actions = Rechercher des actions…
temporary-switch-session = Changer de conversation temporaire
temporary-paste-failed = Réponse copiée. Le collage automatique a échoué. Vérifiez l’autorisation d’accessibilité et l’application cible, ou collez le texte manuellement.
temporary-session-1 = Passer à la conversation temporaire 1
temporary-session-2 = Passer à la conversation temporaire 2
temporary-session-3 = Passer à la conversation temporaire 3
temporary-session-4 = Passer à la conversation temporaire 4
temporary-session-5 = Passer à la conversation temporaire 5
temporary-session-6 = Passer à la conversation temporaire 6
temporary-session-7 = Passer à la conversation temporaire 7
temporary-session-8 = Passer à la conversation temporaire 8
temporary-session-9 = Passer à la dernière conversation temporaire
temporary-send = Envoyer
temporary-trash = Déplacer la conversation temporaire vers la corbeille
settings-key-stop-or-hide = Arrêter la génération / masquer la fenêtre temporaire

conversation-queue-title = En attente ({ $count })
conversation-queue-steer = Orienter le tour en cours
conversation-queue-follow-up = Tâche de suivi
conversation-queue-restore = Récupérer tout le texte en attente dans le brouillon
conversation-queue-clear = Effacer tous les messages en attente
conversation-queue-no-text = Message sans texte
conversation-queue-unavailable = Le contenu de la file d’attente n’est pas encore disponible
conversation-queue-text-only = La récupération de la file restaure uniquement le texte ; les images en attente ne peuvent pas être restaurées.

conversation-retry-countdown = Nouvel essai { $attempt }/{ $total } dans environ { $seconds } s
conversation-retry-waiting = Nouvel essai { $attempt }/{ $total } : en attente de Pi
conversation-summary-retry-countdown = Nouvel essai du résumé { $attempt }/{ $total } dans environ { $seconds } s
conversation-summary-retry-waiting = Nouvel essai du résumé { $attempt }/{ $total } : en attente de Pi

message-details-open = Afficher les détails…
message-details-copy-all = Tout copier
tool-detail-command = Commande
tool-detail-content = Contenu
tool-detail-changes = Modifications

settings-notifications = Notifications
settings-notification-help = Les notifications système utilisent une description générique. Ouvrez une notification pour revenir à sa conversation. Les compteurs de messages non lus sont effacés lorsque la conversation est ouverte au premier plan.
settings-notification-waiting = Notifier lorsqu’une saisie est nécessaire en arrière-plan
settings-notification-failures = Notifier lorsqu’une tâche échoue en arrière-plan
settings-notification-plugins = Envoyer les rappels des extensions comme notifications système en arrière-plan
settings-notification-attention = Demander votre attention lorsqu’une saisie est requise en arrière-plan
settings-notification-completion = Notifications de réponse terminée
notification-off = Désactivé
notification-background = Arrière-plan uniquement
notification-always = Toujours
notification-waiting = En attente de votre saisie
notification-completed = Votre réponse est prête
notification-failed = Une tâche nécessite votre attention après un échec
notification-plugin = Rappels des extensions
notification-unread = Conversations non lues
notification-running = En cours
notification-clear = Effacer les rappels
notification-session-notices = Rappels de conversation

settings-icon-theme = Icône de l’application
settings-icon-theme-help = Modifie le logo dans l’application et l’icône du Dock de l’application macOS en cours d’exécution. Finder et la barre des tâches Windows conservent l’icône empaquetée.
icon-theme-classic = Classique
icon-theme-classic-gradient = Classique dégradé
icon-theme-color = Couleurs Pi
icon-theme-color-gradient = Dégradé Pi
icon-theme-pride = Arc-en-ciel
icon-theme-ukraine = Ukraine
icon-theme-ukraine-gradient = Ukraine dégradé

language-traditional-chinese = 繁體中文
language-japanese = 日本語
language-korean = 한국어
language-german = Deutsch
language-french = Français
language-spanish = Español
language-portuguese-brazil = Português (Brasil)
menu-about = À propos de Gupi
menu-conversation = Conversation
menu-edit = Édition
menu-view = Présentation
menu-window = Fenêtre
menu-help = Aide
menu-services = Services
menu-hide = Masquer Gupi
menu-hide-others = Masquer les autres
menu-show-all = Tout afficher
menu-undo = Annuler
menu-redo = Rétablir
menu-cut = Couper
menu-copy = Copier
menu-paste = Coller
menu-select-all = Tout sélectionner
menu-minimize = Réduire
menu-zoom = Zoom
menu-fullscreen = Activer ou désactiver le plein écran
menu-docs = Guide de l’utilisateur de Gupi
menu-pi-docs = Documentation de Pi
menu-report-issue = Signaler un problème
menu-logs = Afficher les journaux
menu-copy-diagnostics = Copier les diagnostics
settings-permissions = Autorisations système
settings-accessibility-help = La sélection de texte et le collage dans une autre application peuvent nécessiter l’accès à l’accessibilité. Activez Gupi dans Réglages Système, puis réessayez. La réponse reste disponible à copier si le collage échoue.
settings-accessibility-open = Réglages d’accessibilité
settings-notification-permission-help = Si les alertes ou le badge du Dock n’apparaissent pas, autorisez les notifications et les badges de Gupi dans Réglages Système. L’autorisation est demandée lors de la première utilisation de la fonctionnalité.
settings-notification-permission-open = Réglages des notifications
settings-native-language-help = La langue de l’application change immédiatement. Redémarrez Gupi pour l’appliquer aux dialogues système de macOS. Les dialogues système de Windows suivent la langue d’affichage de Windows.
settings-diagnostics-help = Inclut la version de l’application, la plateforme, la commande Pi et les emplacements des fichiers de configuration et de journaux. N’inclut pas les conversations, les identifiants ni les variables d’environnement.

setup-preferences-title = Langue et apparence
setup-preferences-step = Langue et apparence
setup-desktop-title = Raccourcis et notifications
setup-desktop-step = Raccourcis et notifications
setup-desktop-help = Vous pourrez modifier ces choix plus tard dans les réglages.
setup-page-skip = Ignorer cette page
setup-skip-all = Utiliser les valeurs par défaut
setup-pi-later = Configurer Pi plus tard
action-open = Ouvrir

setup-appearance-options = Choisir un thème et une icône
setup-pi-install-guide = Installer Pi
setup-pi-model-guide = Connecter un modèle

conversation-plugin-message = Message du plug-in
conversation-session-info-command = Informations de la conversation…
conversation-session-info-title = Informations de la conversation
conversation-session-info-identity = Identité et emplacement
conversation-session-info-name = Conversation
conversation-session-info-id = ID de conversation Pi
conversation-session-info-directory = Répertoire de travail
conversation-session-info-file = Fichier de conversation
conversation-session-info-not-saved = Pas encore enregistrée
conversation-session-info-runtime = Configuration d’exécution
conversation-session-info-model = Modèle
conversation-session-info-thinking = Niveau de réflexion
conversation-session-info-usage = Utilisation
conversation-session-info-input = Jetons d’entrée
conversation-session-info-output = Jetons de sortie
conversation-session-info-cache-read = Lectures du cache
conversation-session-info-cache-write = Écritures du cache
conversation-session-info-cost = Coût indiqué par Pi
conversation-session-info-context = Contexte actuel (jetons / fenêtre / utilisation)
conversation-session-info-context-value = { $tokens } sur { $window } jetons ({ $percent })
conversation-session-info-counts = Statistiques de la conversation
conversation-session-info-user-messages = Messages utilisateur
conversation-session-info-assistant-messages = Messages de l’assistant
conversation-session-info-tool-calls = Appels d’outils
conversation-session-info-tool-results = Résultats d’outils
conversation-session-info-total-messages = Nombre total de messages

conversation-find = Rechercher dans la conversation
conversation-find-placeholder = Rechercher dans la conversation
conversation-find-count = { $current } sur { $total }
conversation-find-previous = Résultat précédent (Maj+Entrée)
conversation-find-next = Résultat suivant (Entrée)
conversation-find-close = Fermer la recherche (Échap)

# Composer resources
command-select-resource = Insérer dans le message
command-group-files = Fichiers et dossiers
resource-files-loading = Recherche des fichiers et dossiers du projet…
resource-open-file = Ouvrir le fichier
resource-skill-content = Instructions du Skill
resource-template-unreadable = Impossible de lire le modèle de prompt. Rechargez les ressources et réessayez.
resource-template-unused-files = Ce modèle ne reprend pas tous les arguments des fichiers référencés. Ajoutez les paramètres manquants (par exemple, $ARGUMENTS) ou retirez ces références avant l’envoi.
resource-search-files = Rechercher les fichiers et dossiers du projet…
resource-choose-files = Choisir des fichiers ou des dossiers…

resource-edit-in-composer = Choisissez une ressource ici, puis modifiez ses arguments dans le champ du message.
resource-template-unclosed-quote = Fermez les guillemets dans les arguments du modèle avant d’envoyer les références aux fichiers.

resource-answer-required = Choisissez une réponse avant d’envoyer.

menu-check-updates = Rechercher des mises à jour…
updates-title = Mises à jour
updates-check = Rechercher des mises à jour
updates-automatic = Rechercher automatiquement les mises à jour
updates-automatic-help = Recherche les nouvelles versions sur GitHub toutes les heures. Vous choisissez quand télécharger.
updates-idle = Vérifier si une nouvelle version de Gupi est disponible.
updates-checking = Recherche de mises à jour
updates-current = Vous utilisez la dernière version
updates-unpublished = Aucune version stable n’a encore été publiée.
updates-available = Gupi { $version } est disponible
updates-release = Notes de version et téléchargements
updates-install-help = Utilisez le programme d’installation ou le gestionnaire de paquets utilisé pour installer Gupi.
updates-error-network = Impossible de rechercher les mises à jour. Vérifiez votre connexion et réessayez.
updates-error-rate-limit = La limite de requêtes GitHub est atteinte. Réessayez plus tard.
updates-error-response = Impossible de lire les informations de version. Réessayez plus tard.

updates-install = Télécharger et installer…
updates-native-progress = Terminez la mise à jour dans sa fenêtre.
updates-install-failed = Impossible de terminer la mise à jour. Réessayez ou rouvrez Gupi et consultez les journaux.
updates-restart-help = L’installation enregistre les brouillons, ferme les connexions Pi et redémarre Gupi.
updates-show-progress = Afficher la progression de la mise à jour…

shortcut-screenshot = Capture d’écran
screenshot-failed = Impossible de capturer l’écran. Réessayez.
screenshot-permission-required = L’autorisation d’enregistrer l’écran est nécessaire. Autorisez Gupi dans Réglages Système → Confidentialité et sécurité → Enregistrement de l’écran, puis réessayez. Si macOS demande de redémarrer Gupi, faites-le d’abord.

# Project files and source preview
files-title = Fichiers
files-panel = Espace de travail
files-close = Fermer le panneau
files-refresh = Recharger
files-preview = Aperçu
files-readonly = Lecture seule
files-wrap = Retour à la ligne
files-reload = Recharger
files-loading = Lecture…
files-expanded = Développé
files-collapsed = Réduit
files-incomplete = Certaines entrées sont illisibles. Réessayer
files-read-failed = Impossible de lire le fichier ou le dossier. Réessayer
files-unsupported = Ce fichier ne prend pas en charge l’aperçu texte
files-too-large = Limite d’aperçu dépassée (1 MiB ou 20 000 lignes)
files-long-line = Lignes longues : coloration syntaxique désactivée
files-no-session = Sélectionnez une conversation pour parcourir ses fichiers

files-actions = Actions du fichier
files-copy-path = Copier le chemin
files-reveal = Afficher dans les fichiers
files-close-source = Fermer le fichier
files-focus-source = Activer le code source
files-conversation = Conversation
files-conversation-busy = Conversation, réponse en cours
files-view = Vue
files-source = Source

files-link-label = { $name } — lien symbolique
files-link-directory-label = { $name } — dossier lié, { $state }
files-link-unavailable-label = { $name } — cible du lien indisponible
files-link-unavailable = Cible du lien indisponible

# Settings and shortcuts redesign
settings-page-packages = Paquets
settings-app-language = Langue de l’app
settings-config-location = Emplacement
settings-temporary-storage = Stockage des conversations temporaires
settings-accessibility = Accessibilité
settings-open-system-settings = Ouvrir les réglages système
settings-color-mode-label = Mode
settings-light-help-active = Utilisé en mode clair. Actuellement actif.
settings-dark-help-active = Utilisé en mode sombre. Actuellement actif.
settings-icon-theme-help-other = Modifie le logo dans l’app. La barre des tâches et le gestionnaire de fichiers gardent l’icône du paquet.
settings-notification-background-heading = Activité en arrière-plan
settings-notification-completion-heading = Fin de réponse
settings-notification-permission-help-windows = Si les alertes n’apparaissent pas, autorisez les notifications de Gupi dans les Paramètres Windows. L’autorisation est demandée à la première utilisation.
settings-pi-status-ready = Disponible · { $version } · { $path }
settings-pi-draft = Non enregistré
settings-pi-saved-tag = Enregistré
settings-diagnostics = Diagnostic
settings-open-pi = Ouvrir les réglages de Pi
settings-resource-actions = Autres actions
settings-resource-delete-ellipsis = Déplacer vers la corbeille…
settings-resource-from-package = De { $package }
settings-resource-open-package = Afficher le paquet
settings-key-customized = Personnalisé
settings-key-customized-only = Personnalisés uniquement
settings-key-group-system = À l’échelle du système
settings-key-system-help = Ces raccourcis fonctionnent tant que Gupi est ouvert, même lorsqu’une autre app est active.
settings-key-group-workspace = Espace de travail et fichiers
settings-key-reset-all-ellipsis = Rétablir toutes les valeurs par défaut…
settings-key-confirm = Confirmer
settings-key-unsaved = Non enregistré
settings-key-scope-any-window = Toute fenêtre Gupi
settings-key-scope-palette = Fenêtres principale, temporaire et de démarrage
settings-key-scope-conversation = Conversations des fenêtres principale et temporaire
settings-key-scope-messages = Lorsque la liste des messages a le focus
settings-key-scope-stop = Fenêtre principale : arrêter · Fenêtre temporaire : masquer
settings-key-scope-temporary = Fenêtre temporaire
settings-key-scope-temporary-paste = Fenêtre temporaire ; Entrée seule ne s’applique pas dans les champs de texte
settings-key-search = Rechercher des raccourcis
settings-key-search-keystroke = Rechercher par touches
settings-key-search-clear = Effacer la recherche par touches
settings-key-sessions = Passer à la conversation temporaire 1–9
settings-key-sessions-expand = Tout afficher
settings-key-sessions-collapse = Réduire
settings-key-show-conflict = Afficher
settings-key-used-by = Déjà utilisé par « { $command } » ({ $scope })
settings-key-task-actions = Actions de la tâche
settings-key-task-description = /{ $template } · { $source }
settings-key-task-enabled = Activer la tâche
shortcut-delete-ellipsis = Supprimer la tâche modèle…

# Package catalog
packages-installed-tab = Installés
packages-browse-tab = Parcourir
packages-search = Rechercher des paquets
packages-source-npm = Les résultats proviennent des paquets npm portant le mot-clé « pi-package ».
packages-directory = Répertoire des paquets Pi
packages-loading = Chargement…
packages-retry = Réessayer
packages-installed = Installé
packages-previous = Précédent
packages-next = Suivant
packages-back = Retour
packages-latest = Dernière version publiée { $version }
packages-publisher = Publié par { $publisher }
packages-repository = Dépôt
packages-npm = Page npm
packages-install-source = Source d’installation
packages-declared = Contenu déclaré
packages-undeclared = Le paquet ne déclare pas son contenu ; il sera listé après l’installation.
packages-installed-version = { $version } installée
packages-files-missing = Fichiers manquants
packages-trust = Les paquets exécutent du code avec vos autorisations.
packages-done = Terminé. Rechargez les sessions existantes ou ouvrez-en une nouvelle pour appliquer le changement.
packages-failed = Pi a signalé une erreur :
packages-install = Installer
packages-copy-source = Copier la source d’installation
packages-open-repository = Ouvrir le dépôt
packages-open-npm = Ouvrir la page npm
packages-problem-network = Impossible de joindre le registre npm. Vérifiez votre connexion et réessayez.
packages-problem-rate-limited = Le registre npm limite les requêtes. Réessayez plus tard.
packages-problem-not-found = Paquet introuvable.
packages-problem-invalid = Le registre npm a renvoyé une réponse inattendue.
settings-package-install-source = Installer depuis une source…
settings-package-remove-ellipsis = Retirer…

# Package browser layout
packages-source-short = npm · pi-package
packages-page = Page { $page }
packages-page-empty = Aucun paquet correspondant sur cette page.
packages-close = Fermer les détails
packages-local-unavailable = Les informations des paquets locaux sont indisponibles. Actualisez les ressources pour activer les actions sur les paquets.
packages-refresh-details = Recharger

## Pi settings
settings-domain-label = Zone des réglages
settings-domain-gupi = Gupi
settings-domain-pi = Pi
settings-page-connection = Connexion
settings-page-pi-conversation = Conversation
settings-page-pi-network = Réseau
pi-group-model = Modèle par défaut
pi-group-context = Contexte
pi-group-queue = File de messages
pi-group-images = Images
pi-group-proxy = Proxy
pi-group-retry = Nouvelles tentatives
pi-field-model = Modèle par défaut
pi-field-model-help = Modèle par défaut des nouvelles conversations ; le périmètre peut sélectionner un autre modèle initial.
pi-field-model-auto = Automatique
pi-field-model-unavailable = Ce modèle n’est pas disponible actuellement.
pi-field-thinking = Niveau de réflexion par défaut
pi-thinking-unsupported = { $level } (non pris en charge par ce modèle)
pi-field-compaction = Compression automatique
pi-field-reserve = Jetons réservés à la réponse
pi-field-reserve-help = Jetons laissés libres pour la réponse du modèle lors de la compression.
pi-field-keep = Jetons récents conservés
pi-field-keep-help = Contexte récent que la compression ne résume pas.
pi-field-steering = Messages d’orientation
pi-field-follow-up = Messages de suivi
pi-delivery-one = Un par un
pi-delivery-all = Tous à la fois
pi-field-auto-resize = Réduire les grandes images
pi-field-auto-resize-help = Réduit les images à 2000×2000 au maximum avant l’envoi.
pi-field-block-images = Ne pas envoyer d’images au modèle
pi-field-proxy = Proxy HTTP
pi-field-proxy-help = S’applique aux requêtes de Pi, pas à celles de Gupi.
pi-edit-in-global = Modifier dans Global
pi-value-unset = Non défini
pi-field-retry = Nouvelle tentative automatique
pi-field-max-retries = Nombre maximal de tentatives
pi-field-base-delay = Délai initial (ms)
pi-field-max-delay = Délai maximal (ms)
pi-source-default = Par défaut
pi-source-global = Global
pi-source-project = Projet
pi-unsaved = Non enregistré
pi-changed-on-disk = Modifié dans le fichier depuis sa lecture.
pi-invalid-count = Saisissez un nombre entier supérieur ou égal à 0.
pi-menu-revert = Annuler la modification
pi-menu-pin-project = Définir pour ce projet
pi-menu-pin-global = Définir dans les réglages globaux
pi-menu-inherit = Utiliser la valeur globale
pi-menu-default = Utiliser la valeur par défaut
pi-field-actions = Actions du champ
pi-scope-global = Global
pi-scope-label = Portée des réglages
pi-trust-untrusted = Non approuvé
pi-trust-untrusted-help = Pi n’utilisera pas les réglages de ce projet tant que le projet n’est pas approuvé.
pi-trust-action = Approuver le projet…
pi-trust-title = Approuver « { $name } » ?
pi-trust-body = Pi chargera les réglages et ressources de projet de ce dossier, installera ses paquets de projet et exécutera ses extensions de projet. Les dossiers parents ne sont pas approuvés.
pi-trust-confirm = Approuver le projet
pi-settings-save = Enregistrer
pi-settings-discard = Abandonner
pi-settings-file-actions = Autres actions
pi-settings-open-file = Ouvrir settings.json
pi-settings-reload = Recharger
pi-settings-invalid = Corrigez les champs signalés avant d’enregistrer.
pi-settings-conflict = Certains réglages ont été modifiés dans le fichier après leur lecture. Rechargez, puis enregistrez à nouveau.
pi-settings-locked = Le fichier de réglages est utilisé par un autre processus. Réessayez.
pi-settings-unsaved-title = Enregistrer les réglages Pi ?
pi-settings-unsaved-body = Vos modifications des réglages Pi n’ont pas été enregistrées.
pi-settings-loading = Chargement…
pi-settings-unsupported = La modification des réglages Pi ici nécessite Pi 1.1.0 ou une version ultérieure. Version installée : { $version }.
pi-settings-unavailable = Pi n’est pas disponible. Vérifiez la page Connexion.
pi-models-loading = Chargement des modèles…
pi-models-failed = Impossible de charger les modèles : { $error }
pi-models-retry = Réessayer
pi-project-settings = Réglages Pi du projet…

pi-scope-choose-folder = Choisir le dossier du projet…
pi-scope-choose-folder-action = Choisir
settings-resource-group-project = Ce projet
settings-resource-group-inherited = Hérité du global
settings-source-project = Ce projet
settings-resource-same-global = Même nom qu’en global · le projet d’abord
settings-resource-same-project = Même nom que dans le projet · le projet d’abord
settings-resource-not-loaded = Non chargé (projet non approuvé)
settings-resource-edit-global = Modifier en global
settings-resource-none-project = Le dossier .pi de ce projet n’en contient pas encore.
settings-system-project-file = Fichier du projet
settings-system-uses-global = Utilise le global
settings-system-use-global = Utiliser le global
settings-system-use-global-title = Revenir au { $file } global ?
settings-system-use-global-body = Le fichier du projet est placé dans la corbeille et Pi revient au fichier global.
settings-editor-changed = Le fichier a changé après son ouverture : il n’a pas été enregistré. Relisez-le pour abandonner ces modifications, ou annulez.
settings-editor-reload = Recharger
settings-editor-unsaved-title = Enregistrer les modifications de { $file } ?
settings-editor-unsaved-body = Vos modifications de texte ne sont pas encore enregistrées.

pi-project-settings-folder = Paramètres Pi du projet — { $folder }

packages-install-project = Installer dans le projet { $folder }
packages-project-delta = Filtres du paquet global · lecture seule
packages-project-filtered = Filtré dans ce projet
packages-project-replaced = Remplacé par le projet
packages-project-replaces = Remplace le paquet global

settings-resource-disabled = Désactivé
settings-theme-heading = Thèmes indépendants
settings-project-package-empty = Aucun paquet dans ce projet.

pi-model-scope = Périmètre des modèles
pi-model-scope-help = Les nouvelles conversations utilisent le modèle par défaut s’il appartient à ce périmètre, sinon le premier modèle correspondant. Le choix de modèle dans une conversation reste libre.
pi-model-scope-add-model = Ajouter un modèle…
pi-model-scope-add-pattern = Ajouter un motif
pi-model-scope-up = Monter
pi-model-scope-down = Descendre
pi-model-scope-delete = Supprimer le motif
pi-model-scope-unrestricted = Sans restriction
pi-model-scope-unrestricted-project = Sans restriction (remplace le réglage global)
pi-model-scope-blank = Les motifs ne peuvent pas être vides.
pi-model-scope-malformed = Cette valeur doit être un tableau de chaînes. Corrigez-la dans le fichier de configuration.

conversation-edited-files = { $count ->
    [one] { $count } fichier modifié
   *[other] { $count } fichiers modifiés
    }
conversation-edit-write = Écriture
conversation-edit-error = Erreur
conversation-written-content = Contenu écrit
conversation-edit-operation = Opération { $index }
conversation-edit-more-files = { $count ->
    [one] Afficher { $count } fichier de plus
   *[other] Afficher { $count } fichiers de plus
    }
conversation-edit-less-files = Afficher moins de fichiers
conversation-edit-more-operations = { $count ->
    [one] Afficher { $count } opération de plus
   *[other] Afficher { $count } opérations de plus
    }
conversation-edit-previous-operations = Opérations précédentes
conversation-edit-stats-tooltip = Lignes nettes ajoutées et supprimées des correctifs enregistrés combinés. Les écritures et les fichiers non combinables sont exclus ; ce résultat ne couvre pas tout l’espace de travail.
conversation-edit-file-label = { $path } — { $state }
conversation-edit-operation-stats-tooltip = Lignes ajoutées et supprimées dans ce correctif enregistré.
conversation-edit-uncombined = Combinaison impossible ; affichage par opération

# Advanced Pi settings
settings-page-pi-tools = Outils et environnement
pi-group-tools = Outils
pi-group-shell = Shell
pi-group-package-command = Gestionnaire de paquets
pi-group-privacy-trust = Confidentialité et confiance
pi-group-advanced = Avancé
pi-group-provider = Tentatives du fournisseur
pi-field-default-tools = Outils par défaut
pi-field-codemode-mode = Mode Codemode
pi-field-codemode-budget = Budget des déclarations
pi-field-shell-path = Chemin du shell
pi-field-shell-prefix = Préfixe de commande
pi-field-npm-command = Commande npm
pi-field-telemetry = Télémétrie d’installation
pi-field-project-trust = Confiance par défaut des projets
pi-field-model-thinking = Niveaux de réflexion par modèle
pi-field-model-overrides = Compactage par modèle
pi-field-budget-minimal = Minimal
pi-field-budget-low = Faible
pi-field-budget-medium = Moyen
pi-field-budget-high = Élevé
pi-field-cache-warming = Préchauffage du cache
pi-field-transport = Transport
pi-field-http-timeout = Délai d’inactivité HTTP (ms)
pi-field-websocket-timeout = Délai de connexion WebSocket (ms)
pi-field-provider-timeout = Délai de requête (ms)
pi-field-provider-retries = Nombre de tentatives
pi-field-provider-delay = Délai maximal de nouvelle tentative (ms)
pi-timeout-disabled = Désactiver
pi-tools-startup-help = Les extensions et MCP peuvent activer d’autres outils. Ce n’est ni une limite de sécurité ni une liste en temps réel.
pi-map-add-model = Ajouter un modèle
pi-map-remove-local = Supprimer la valeur locale
pi-model-thinking-help = Au démarrage, le niveau explicite et la session priment ; changer de modèle utilise cette valeur. Un suffixe de réflexion est prioritaire.
pi-model-overrides-help = Chaque valeur de tokens est héritée séparément. Seuls les champs modifiés sont enregistrés.
pi-trust-always-title = Faire confiance aux projets par défaut ?
pi-trust-always-body = Pi fera confiance aux projets sans décision applicable, chargera leurs réglages et exécutera leurs extensions. Les décisions explicites et extensions de confiance restent prioritaires.
pi-telemetry-env-on = PI_TELEMETRY remplace cette préférence : activé
pi-telemetry-env-off = PI_TELEMETRY remplace cette préférence : désactivé
pi-codemode-help = Les réglages codemode nécessitent l’extension intégrée codemode et son outil activés.
pi-thinking-budget-help = S’applique uniquement aux fournisseurs et modèles compatibles avec les budgets de tokens.
pi-cache-warming-help = Les requêtes de préchauffage entraînent une consommation.
pi-telemetry-help = Contrôle la télémétrie d’installation, pas les mises à jour ni toutes les analyses.
pi-provider-retry-help = Pi recommande de conserver 0.
pi-trust-always-confirm = Faire confiance par défaut
pi-choice-ask = Demander
pi-choice-always = Toujours faire confiance
pi-choice-never = Ne jamais faire confiance
pi-choice-codemode-on = Outils directs et Codemode
pi-choice-codemode-only = Codemode uniquement
pi-choice-streaming = Pendant la diffusion
pi-choice-idle = Également au repos
pi-choice-auto = Automatique

pi-thinking-budgets = Budgets de réflexion (tokens)
pi-model-unavailable-short = Indisponible
pi-models-unavailable = Impossible de charger la liste des modèles
pi-map-model = Modèle
pi-map-reserve = Tokens réservés
pi-map-keep = Tokens récents
pi-map-token-action = { $field } : { $action }
pi-tools-add = Ajouter
pi-tools-custom = Personnalisé…
pi-tool-remove = Retirer { $tool }
pi-tools-preview = Au démarrage : { $tools }
pi-npm-invalid = Saisissez une commande avec des guillemets fermés et un premier argument non vide.
pi-timeout-off = Désactivé
pi-npm-help = Les guillemets du shell préservent les espaces et les arguments vides.
pi-budget-input = Budget de réflexion { $level } (tokens)

pi-group-resource-paths = Chemins des ressources
pi-field-resource-extensions = Extensions
pi-field-resource-skills = Compétences
pi-field-resource-prompts = Prompts
pi-field-resource-themes = Thèmes
pi-resource-paths-help = Entrées brutes du fichier de cette portée, dans leur ordre.
pi-resource-unsupported = Format non pris en charge · lecture seule
pi-resource-absent = Clé absente
pi-resource-empty = Tableau vide []
pi-resource-up = Monter
pi-resource-down = Descendre
pi-resource-remove = Supprimer l’entrée
pi-resource-add = Ajouter
pi-resource-clear = Vider le tableau []
pi-resource-remove-key = Supprimer la clé
pi-resource-entry-actions = Actions sur l’entrée

# Pi configuration files
pi-files-models = Modèles et fournisseurs
pi-files-mcp = MCP
pi-files-startup-only = Pi lit ce fichier au démarrage d’une nouvelle session. Les sessions existantes ne sont pas rechargées.
pi-files-global-only = Ce fichier est uniquement global. Sélectionnez la portée globale pour le modifier.
pi-files-comments = L’enregistrement supprime les commentaires du fichier.
pi-files-add = Ajouter…
pi-files-edit = Modifier…
pi-files-delete = Supprimer
pi-files-override = Surcharge
pi-files-definition = Définition complète
pi-files-disabled = Désactivé
pi-files-duplicate = Ce nom entre en conflit avec une entrée existante.
pi-files-invalid = Impossible de lire ou d’enregistrer cette entrée. Vérifiez les champs et le fichier de configuration.
pi-files-secret-set = Défini · conserver
pi-files-secret-keep = Conserver
pi-files-secret-replace = Remplacer par une nouvelle valeur
pi-files-secret-remove = Retirer
pi-files-name = Nom
pi-files-key = Clé
pi-files-model = Modèle
pi-files-add-model = Ajouter un modèle
pi-files-on = Activé
pi-files-off = Désactivé

# Structured configuration editor
pi-files-label-name = Nom affiché
pi-files-label-id = ID du modèle
pi-files-label-baseUrl = URL de base
pi-files-label-api = API
pi-files-label-apiKey = Clé API
pi-files-label-authHeader = En-tête d’autorisation
pi-files-label-oauth = OAuth
pi-files-label-compat = Compatibilité
pi-files-label-models = Modèles
pi-files-label-modelOverrides = Surcharges de modèles
pi-files-label-enabled = Activé
pi-files-label-exposure = Exposition
pi-files-label-type = Transport
pi-files-label-description = Description
pi-files-label-command = Commande
pi-files-label-url = URL
pi-files-label-cwd = Dossier de travail
pi-files-label-timeout = Délai d’attente
pi-files-label-toolExposure = Exposition des outils
pi-files-label-args = Arguments
pi-files-label-env = Variables d’environnement
pi-files-label-headers = En-têtes
pi-files-label-clientId = ID du client
pi-files-label-clientSecret = Secret du client
pi-files-label-callbackPort = Port de rappel
pi-files-label-callbackUrl = URL de rappel
pi-files-label-scope = Autorisations
pi-files-label-clientName = Nom du client
pi-files-label-clientRegistration = Enregistrement du client
pi-files-label-authServerMetadataUrl = URL des métadonnées d’autorisation
pi-files-label-provider = Fournisseur
pi-files-label-reasoning = Raisonnement
pi-files-label-contextWindow = Fenêtre de contexte
pi-files-label-maxTokens = Tokens de sortie maximum
pi-files-label-input = Types d’entrée
pi-files-label-cost = Coût
pi-files-label-thinkingLevelMap = Correspondance des niveaux de réflexion
pi-files-label-inputLimits = Limites d’entrée
pi-files-label-promptCache = Cache de prompts
pi-files-label-samplingParams = Paramètres d’échantillonnage
pi-files-label-samplingParamsByThinkingLevel = Échantillonnage par niveau de réflexion
pi-files-label-autoEnableCodemode = Activer Codemode automatiquement
pi-files-providers = Fournisseurs
pi-files-servers = Serveurs MCP
pi-files-automation = Codemode
pi-files-empty = Aucune entrée
pi-files-basic = Général
pi-files-connection = Connexion
pi-files-exposure-section = Exposition
pi-files-authentication = Authentification
pi-files-advanced = Avancé
pi-files-back = Retour
pi-files-stage = Préparer les modifications
pi-files-secret-present = Défini
pi-files-add-value = Nouvelle valeur
pi-files-confirm-add = Ajouter
pi-files-definition-kind = Définition
pi-files-provider-summary =
    { $models ->
        [one] { $models } modèle
       *[other] { $models } modèles
    } · { $overrides ->
        [one] { $overrides } surcharge
       *[other] { $overrides } surcharges
    }

# Pi provider authentication
pi-auth-title = Authentification
pi-auth-help = Les identifiants sont stockés dans auth.json de Pi. Les changements prennent pleinement effet dans les nouvelles sessions. Gupi ne reconnecte pas les sessions existantes et ne synchronise pas leurs listes de modèles.
pi-auth-add = Ajouter une clé API…
pi-auth-replace = Remplacer la clé API…
pi-auth-api-key = Clé API
pi-auth-oauth = OAuth
pi-auth-unknown = Non reconnu
pi-auth-missing = Aucun identifiant enregistré
pi-auth-set = Déjà définie
pi-auth-empty = Aucun fournisseur enregistré. Ajoutez une clé API pour commencer.
pi-auth-provider = ID du fournisseur
pi-auth-key-placeholder = Saisir une nouvelle clé API
pi-auth-saving = Enregistrement…
pi-auth-candidates = Choisir un fournisseur
pi-auth-check = Vérifier la configuration
pi-auth-checking = Vérification de la configuration
pi-auth-ready = Pi a trouvé une configuration utilisable. Cela ne valide pas le jeton.
pi-auth-not-ready = Pi n’a trouvé aucune configuration utilisable.
pi-auth-invalid = Pi a signalé une configuration invalide.
pi-auth-check-failed = Impossible de vérifier la configuration.
pi-auth-provider-not-found = Fournisseur introuvable.
pi-auth-not-configured = Identifiants non configurés.
pi-auth-not-available = Identifiants indisponibles.
pi-auth-invalid-state = Pi n’a pas pu lire l’état d’authentification.
pi-auth-copy-login = Copier la commande de connexion
pi-auth-login-help = Commande copiée. Exécutez-la dans le terminal Pi pour vous reconnecter.
pi-auth-oauth-help = Enregistrement local uniquement. Pi peut tenter de renouveler les identifiants lors de leur utilisation. Reconnectez-vous si nécessaire.
pi-auth-expired = Date d’expiration enregistrée dépassée
pi-auth-expires = Expiration enregistrée : { $time }
pi-auth-remove = Supprimer les identifiants…
pi-auth-logout = Se déconnecter…
pi-auth-remove-title = Supprimer les identifiants de « { $provider } » ?
pi-auth-logout-title = Se déconnecter de « { $provider } » ?
pi-auth-remove-help = Seuls les identifiants locaux sont supprimés. Les jetons du serveur ne sont pas révoqués. Les variables d’environnement ou apiKey dans models.json peuvent toujours authentifier.
pi-auth-remove-env = Les valeurs d’environnement et de configuration du fournisseur seront également supprimées.
pi-auth-remove-confirm = Supprimer
pi-auth-logout-confirm = Se déconnecter
pi-auth-replace-title = Remplacer les identifiants de « { $provider } » ?
pi-auth-replace-help = L’identifiant entier, y compris les jetons et les champs inconnus, sera remplacé par une clé API.
pi-auth-replace-confirm = Remplacer
pi-auth-read-error = Impossible de lire auth.json.
pi-auth-parse-error = auth.json doit contenir un objet JSON valide. La modification est désactivée.
pi-auth-busy = auth.json est occupé. Réessayez.
pi-auth-conflict = Ce fournisseur a changé sur le disque. Rechargez puis réessayez.
pi-auth-write-error = Impossible d’écrire auth.json. Rechargez pour vérifier l’état actuel.
pi-auth-input-error = Choisissez un fournisseur ou saisissez un ID non vide sur une seule ligne, puis une clé API.
pi-auth-login-select-help = Commande copiée. Exécutez /login dans le terminal Pi et choisissez le fournisseur.

# Authentication and model provider settings
pi-files-auth-stored = Identifiants enregistrés dans Authentification
pi-files-manage-auth = Gérer dans Authentification
pi-files-api-key-auth-help = Si des identifiants sont enregistrés dans Authentification, Pi n’utilise pas cet apiKey en secours. Des identifiants enregistrés ne garantissent pas une authentification valide.
pi-auth-models-help = Les identifiants enregistrés influencent l’utilisation de apiKey dans models.json. Pi gère l’authentification selon les règles du fournisseur.
pi-auth-edit-models = Modifier dans Modèles et fournisseurs

pi-provider-empty-id = ⟨ID vide⟩
pi-auth-check-unavailable = Pi ne peut pas vérifier cet ID de fournisseur.

settings-skill-name-help = Nom du dossier. Pi recommande des minuscules, des chiffres et des traits d’union simples, sur 64 caractères maximum.

settings-skill-name-warning = Ce nom ne suit pas les recommandations de Pi ; sa création reste possible. Le champ name de SKILL.md n’est pas vérifié ici.

settings-prompt-name-help = Utilisé avec /nom. Les espaces ne sont pas autorisés.

settings-prompt-invalid-name = Utilisez un nom de fichier valide sans espaces ni séparateurs de chemin.

shortcut-add-title = Ajouter une tâche de modèle

shortcut-edit-title = Modifier la tâche de modèle

shortcut-options-failed = Impossible de charger les modèles ou leurs options. Vérifiez la connexion à Pi et réessayez.

pi-auth-row-action = { $provider }: { $action }

pi-auth-add-title = Ajouter une clé API

pi-auth-replace-editor-title = Remplacer la clé API

temporary-clean-title = Nettoyer les répertoires de travail libérés ?

settings-package-remove-title = Retirer « { $name } » ?

settings-resource-delete-title = Déplacer « { $name } » dans la corbeille ?

shortcut-delete-title = Supprimer la tâche de modèle « { $name } » ?

settings-key-control-label = { $command }: { $binding }
settings-key-control-recording = { $command }: { $binding } — { $state }

# MCP CLI login and last-check results
pi-mcp-help = Ces actions concernent le MCP intégré de Pi. Les extensions peuvent avoir d’autres états de session. Les services utilisant auth.provider peuvent demander une connexion ici tout en fonctionnant dans les sessions.
pi-mcp-reconnect = Pour charger toute la configuration MCP enregistrée dans une session active, choisissez Reconnecter dans le menu de conversation. Les nouvelles sessions la chargent automatiquement.
pi-mcp-global-cwd = La vérification globale utilise un dossier temporaire vide ; les chemins relatifs peuvent se comporter différemment dans les sessions.
pi-mcp-save-first = Enregistrez ou abandonnez les modifications
pi-mcp-check = Vérifier la connexion
pi-mcp-running = En cours…
pi-mcp-last-check = Dernière vérification { $time }
pi-mcp-config-error = La configuration contient des erreurs
pi-mcp-untrusted = Projet non approuvé ; Pi utilise uniquement la configuration globale
pi-mcp-resources = Ressources disponibles
pi-mcp-connected = { $count ->
    [one] Connecté ({ $count } outil)
   *[other] Connecté ({ $count } outils)
    }
pi-mcp-needs-auth = Connexion requise
pi-mcp-connection-failed = Échec de connexion
pi-mcp-disconnected = Non connecté
pi-mcp-not-checked = Non vérifié
pi-mcp-unsupported-name = Pi ne peut pas connecter ou déconnecter ce nom via la CLI
pi-mcp-login = Se connecter
pi-mcp-logout = Se déconnecter
pi-mcp-login-title = Se connecter au serveur MCP « { $name } »
pi-mcp-logout-title = Se déconnecter du serveur MCP « { $name } » ?
pi-mcp-login-help = Si nécessaire, Pi ouvre un navigateur. Terminez la connexion dans celui-ci.
pi-mcp-logout-help = Supprime les identifiants locaux sans révoquer l’autorisation du serveur ni déconnecter immédiatement les sessions actives. Reconnectez ces sessions pour actualiser leurs connexions.
pi-mcp-login-terminal = Si aucun navigateur ne s’ouvre, annulez puis exécutez pi mcp login -- { $name } dans un terminal.
pi-mcp-completed = Commande terminée
pi-mcp-cancelled = Annulé
pi-mcp-timeout = Délai dépassé ; résultats incomplets
pi-mcp-login-failed = Échec de connexion. Les identifiants ont pu être enregistrés avant l’échec. Vérifiez la connexion.
pi-mcp-failed = Échec de l’opération
pi-mcp-close = Fermer
pi-mcp-scope-project = Projet
pi-mcp-disabled = Désactivé
pi-mcp-check-failed = Échec de la vérification
pi-mcp-check-timeout = Délai de vérification dépassé ; résultats incomplets

workspace-changes = Modifications
workspace-close-tab = Fermer l’onglet
workspace-close-others = Fermer les autres onglets
workspace-close-right = Fermer les onglets à droite
workspace-toggle-file-tree = Afficher ou masquer l’arborescence
workspace-toggle-change-list = Afficher ou masquer les fichiers modifiés
workspace-out-of-scope = Cette réponse est hors du périmètre actuel.
workspace-net-zero = Toutes les modifications de cette réponse s’annulent.
workspace-open-file = Ouvrir le fichier
workspace-next-operations = Opérations suivantes
workspace-open-change = Ouvrir { $path } dans l’espace de travail

history-panel-title = Historique
history-panel-options = Options de l’historique
workspace-view-changes = Voir les modifications

workspace-new-tab = Nouvel onglet
workspace-all-tabs = Tous les onglets
workspace-tools = Outils
workspace-recent = Récents
workspace-no-changes = Cette conversation ne contient pas encore de modifications.
workspace-recent-unavailable = Ces modifications ne sont pas disponibles dans le périmètre actuel.
workspace-just-now = À l’instant
workspace-minutes-ago = Il y a { $count } min
workspace-maximize = Agrandir l’espace de travail
workspace-restore = Restaurer l’espace de travail

# Direct Shell command output
shell-title = Shell
shell-running = Commande en cours
shell-stopping = Arrêt de la commande
shell-stop = Arrêter la commande
shell-cancelled = Annulée
shell-exit-code = Code de sortie { $code }
shell-completed = Terminée · état de sortie indisponible
shell-excluded = Exclue du contexte du modèle
shell-show-all = Afficher toute la sortie
shell-show-less = Réduire la sortie
shell-open-full-output = Ouvrir la sortie complète
shell-display-truncated = Seule la sortie la plus récente est affichée.

titlebar-git-branch = Branche Git : { $name }
titlebar-git-detached = HEAD détachée : { $name }

# Diff reader
diff-worktree = Git
diff-unstaged = Non indexées
diff-staged = Indexées
diff-untracked = Non suivies
diff-filter = Filtrer les chemins…
diff-previous-file = Fichier précédent
diff-next-file = Fichier suivant
diff-unified = Unifié
diff-split = Côte à côte
diff-wrap = Retour à la ligne
diff-actions = Actions du fichier
diff-word = Différences par mot
diff-context = Toutes les lignes inchangées
diff-expand-file = Déplier ce fichier
diff-copy-path = Copier le chemin du fichier
diff-copy-patch = Copier le patch
diff-copy-content = Copier le texte intégral du fichier
diff-external = Ouvrir dans un éditeur externe
diff-refresh = Actualiser les modifications
diff-refresh-failed = Impossible d’actualiser les modifications. Les résultats précédents peuvent être périmés.
diff-loading = Chargement des modifications…
diff-not-repository = Ce dossier n’appartient pas à un arbre de travail Git
diff-empty = Aucune modification
diff-no-results = Aucun fichier correspondant
diff-retry = Réessayer
diff-recorded-context = Modifications de cette exécution · contexte enregistré uniquement
diff-added = Ajouté
diff-deleted = Supprimé
diff-modified = Modifié
diff-conflict = Conflit · lecture seule
diff-special = Fichier spécial · lecture seule
diff-binary = Fichier binaire
diff-too-large = Ce fichier dépasse les limites du lecteur de texte
diff-empty-file = Fichier vide
diff-unavailable-file = Le fichier actuel est indisponible ; l’ouverture externe est désactivée.
diff-git-unavailable = Git est indisponible. Installez Git puis actualisez.
diff-files = Fichiers
diff-conflicts = Conflits
diff-direction-staged = HEAD → index
diff-direction-unstaged = Index → arbre de travail
diff-direction-untracked = Vide → arbre de travail
diff-updating = Actualisation de la version précédente…
diff-collapse-file = Replier ce fichier
diff-update-failed = Cette version n’a pas pu être actualisée
diff-reading-options = Options de lecture
diff-reading-options-narrow = Options de lecture : la zone est trop étroite pour la vue côte à côte ; son élargissement rétablit la vue préférée
diff-view = Vue
diff-display = Affichage
diff-list-view = Liste
diff-tree-view = Arborescence
diff-stats-incomplete = Statistiques incomplètes

workspace-back-conversation = Revenir à la conversation
workspace-back-content = Revenir au contenu
workspace-open = Ouvrir l’espace de travail
workspace-close = Fermer l’espace de travail

diff-invalid-path = Le nom du fichier n’est pas en UTF-8 valide et ne peut pas être lu
diff-no-text-changes = Aucune modification de texte

diff-external-failed = Impossible d’ouvrir le fichier. Il n’existe peut-être plus ou aucune application adaptée n’est disponible.

diff-conflict-label = Conflit
diff-read-failed = Impossible de lire les modifications. Réessayez.
