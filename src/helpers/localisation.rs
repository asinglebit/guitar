#![allow(non_snake_case)]

use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU8, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    English = 0,
    Spanish = 1,
    French = 2,
    Russian = 3,
    Turkish = 4,
}

impl Language {
    pub const ALL: [Language; 5] = [Language::English, Language::Spanish, Language::French, Language::Russian, Language::Turkish];

    pub const fn id(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Spanish => "spanish",
            Language::French => "french",
            Language::Russian => "russian",
            Language::Turkish => "turkish",
        }
    }

    pub const fn native_label(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::Russian => "Русский",
            Language::Turkish => "Türkçe",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        let normalized = value.trim().to_lowercase().replace(['-', '_'], " ");
        match normalized.as_str() {
            "en" | "eng" | "english" => Some(Language::English),
            "es" | "spa" | "spanish" | "espanol" | "español" => Some(Language::Spanish),
            "fr" | "fr fr" | "fre" | "fra" | "french" | "francais" | "français" => Some(Language::French),
            "ru" | "rus" | "russian" | "русский" => Some(Language::Russian),
            "tr" | "tur" | "turkish" | "turkce" | "türkçe" => Some(Language::Turkish),
            _ => None,
        }
    }

    fn from_index(index: u8) -> Self {
        match index {
            1 => Language::Spanish,
            2 => Language::French,
            3 => Language::Russian,
            4 => Language::Turkish,
            _ => Language::English,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

static ACTIVE_LANGUAGE: AtomicU8 = AtomicU8::new(Language::English as u8);

pub fn active_language() -> Language {
    Language::from_index(ACTIVE_LANGUAGE.load(Ordering::Relaxed))
}

pub fn set_active_language(language: Language) {
    ACTIVE_LANGUAGE.store(language as u8, Ordering::Relaxed);
}

fn language_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("language.json");
    path
}

pub fn load_language_from_path(path: &Path) -> Language {
    if let Ok(contents) = fs::read_to_string(path) {
        if let Ok(language_id) = facet_json::from_str::<String>(&contents)
            && let Some(language) = Language::from_id(&language_id)
        {
            return language;
        }
    }

    Language::English
}

pub fn save_language_to_path(path: &Path, language: Language) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, facet_json::to_string_pretty(&language.id().to_string())?)?;
    Ok(())
}

pub fn load_language() -> Language {
    load_language_from_path(&language_path())
}

pub fn save_language(language: Language) -> Result<(), Box<dyn std::error::Error>> {
    save_language_to_path(&language_path(), language)
}

fn tr(en: &'static str) -> &'static str {
    match active_language() {
        Language::English => en,
        Language::Spanish => es(en),
        Language::French => fr(en),
        Language::Russian => ru(en),
        Language::Turkish => tr_tr(en),
    }
}

pub fn command_label(en: &'static str) -> &'static str {
    tr(en)
}

fn es(en: &'static str) -> &'static str {
    match en {
        "default" => "predeterminado",
        "loading" => "cargando",
        "none" => "ninguno",
        "no head" => "sin HEAD",
        "not initialized" => "no inicializado",
        "working..." => "trabajando...",
        "no body" => "sin cuerpo",
        "no branches" => "sin ramas",
        "no commits" => "sin commits",
        "no HEAD reflog" => "sin reflog de HEAD",
        "no message" => "sin mensaje",
        "no recent repositories" => "sin repositorios recientes",
        "no remotes" => "sin remotos",
        "no staged changes" => "sin cambios preparados",
        "no stashes" => "sin stashes",
        "no submodules" => "sin submódulos",
        "no summary" => "sin resumen",
        "no tags" => "sin etiquetas",
        "no unstaged changes" => "sin cambios no preparados",
        "no worktrees" => "sin worktrees",
        "search" => "buscar",
        "Abort failed: no rebase, cherry-pick, revert, or merge in progress" => "Abortar falló: no hay rebase, cherry-pick, revert o merge en curso",
        "Add remote failed: remote name is invalid" => "Añadir remoto falló: el nombre del remoto no es válido",
        "Add remote failed" => "Añadir remoto falló",
        "Checkout failed" => "Checkout falló",
        "Cherry-pick failed" => "Cherry-pick falló",
        "Cherry-pick failed: no commit message was provided" => "Cherry-pick falló: no se proporcionó mensaje de commit",
        "Cherry-pick failed: no commit is pending" => "Cherry-pick falló: no hay ningún commit pendiente",
        "Commit failed" => "Commit falló",
        "Continue failed: no rebase, cherry-pick, revert, or merge in progress" => "Continuar falló: no hay rebase, cherry-pick, revert o merge en curso",
        "Create branch failed" => "Crear rama falló",
        "Create branch failed: no commit is selected" => "Crear rama falló: no hay ningún commit seleccionado",
        "Create tag failed" => "Crear etiqueta falló",
        "Create tag failed: no commit is selected" => "Crear etiqueta falló: no hay ningún commit seleccionado",
        "Create worktree failed" => "Crear worktree falló",
        "Create worktree failed: names cannot be empty or contain path separators" => "Crear worktree falló: los nombres no pueden estar vacíos ni contener separadores de ruta",
        "Create worktree failed: path cannot be empty" => "Crear worktree falló: la ruta no puede estar vacía",
        "Create worktree failed: no commit is selected" => "Crear worktree falló: no hay ningún commit seleccionado",
        "Delete branch failed" => "Eliminar rama falló",
        "Delete branch failed: cannot delete the current branch" => "Eliminar rama falló: no se puede eliminar la rama actual",
        "Delete branch failed: remote branch name is invalid" => "Eliminar rama falló: el nombre de la rama remota no es válido",
        "Delete remote failed" => "Eliminar remoto falló",
        "Delete remote failed: no remote is pending" => "Eliminar remoto falló: no hay ningún remoto pendiente",
        "Delete tag failed" => "Eliminar etiqueta falló",
        "Drop stash failed" => "Descartar stash falló",
        "Edit remote failed" => "Editar remoto falló",
        "Edit remote failed: no remote is pending" => "Editar remoto falló: no hay ningún remoto pendiente",
        "Couldn't get the file diff" => "No se pudo obtener el diff del archivo",
        "File history failed: graph worker is unavailable" => "Historial del archivo falló: el trabajador del grafo no está disponible",
        "Git network operation failed: another network operation is already running" => "Operación de red de Git falló: ya hay otra operación de red en curso",
        "Git network operation failed: worker thread panicked" => "Operación de red de Git falló: el hilo de trabajo entró en pánico",
        "Git operation failed: no repository is open" => "Operación de Git falló: no hay ningún repositorio abierto",
        "Hard reset failed" => "Hard reset falló",
        "Lock worktree failed" => "Bloquear worktree falló",
        "Lock worktree failed: only valid linked worktrees can be locked" => "Bloquear worktree falló: solo se pueden bloquear worktrees vinculados válidos",
        "Merge failed" => "Merge falló",
        "Mixed reset failed" => "Mixed reset falló",
        "Open repository failed" => "Abrir repositorio falló",
        "Open submodule failed: submodule is not initialized. Run update/init first." => "Abrir submódulo falló: el submódulo no está inicializado. Ejecuta update/init primero.",
        "Open worktree failed: worktree path is invalid" => "Abrir worktree falló: la ruta del worktree no es válida",
        "Pop stash failed" => "Aplicar stash falló",
        "Push failed: detached HEAD has no current branch" => "Push falló: HEAD desacoplado no tiene rama actual",
        "Rebase failed" => "Rebase falló",
        "Remove worktree failed" => "Eliminar worktree falló",
        "Remove worktree failed: cannot remove current, main, or locked worktrees" => "Eliminar worktree falló: no se pueden eliminar worktrees actual, principal o bloqueados",
        "Rename branch failed" => "Renombrar rama falló",
        "Rename branch failed: only local branches can be renamed" => "Renombrar rama falló: solo se pueden renombrar ramas locales",
        "Rename branch failed: no branch is pending" => "Renombrar rama falló: no hay ninguna rama pendiente",
        "Rename remote failed" => "Renombrar remoto falló",
        "Rename remote failed: no remote is pending" => "Renombrar remoto falló: no hay ningún remoto pendiente",
        "Reflog commit is hidden from the graph. Press 9 to show graph reflogs." => "El commit del reflog está oculto en el grafo. Pulsa 9 para mostrar reflogs.",
        "Reset file failed" => "Restablecer archivo falló",
        "Revert failed" => "Revert falló",
        "Revert failed: reverting merge commits is not supported" => "Revert falló: revertir commits de merge no está soportado",
        "Revert failed: no commit message was provided" => "Revert falló: no se proporcionó mensaje de commit",
        "Revert failed: no commit is pending" => "Revert falló: no hay ningún commit pendiente",
        "Save keymap failed" => "Guardar mapa de teclas falló",
        "Set default remote failed" => "Establecer remoto predeterminado falló",
        "Stage all failed" => "Preparar todo falló",
        "Stage file failed" => "Preparar archivo falló",
        "Stage file failed: resolve conflicts in your editor, then continue the active operation" => "Preparar archivo falló: resuelve conflictos en tu editor y continúa la operación activa",
        "Stage submodule failed" => "Preparar submódulo falló",
        "Stash failed" => "Stash falló",
        "Sync submodule failed" => "Sincronizar submódulo falló",
        "Unstage all failed" => "Quitar todo del índice falló",
        "Unstage file failed" => "Quitar archivo del índice falló",
        "Unstage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Quitar archivo del índice falló: resuelve conflictos en tu editor y continúa la operación activa"
        },
        "Unstage submodule failed" => "Quitar submódulo del índice falló",
        "Unlock worktree failed" => "Desbloquear worktree falló",
        "authored by:" => "autor:",
        "commit sha:" => "sha del commit:",
        "committed by:" => "commiteado por:",
        "conflicted files:" => "archivos en conflicto:",
        "featured branches:" => "ramas destacadas:",
        "head reflog:" => "reflog de HEAD:",
        "message body:" => "cuerpo del mensaje:",
        "message summary:" => "resumen del mensaje:",
        "next action:" => "siguiente acción:",
        "operation conflicts" => "conflictos de operación",
        "parent shas:" => "shas padre:",
        "repository state:" => "estado del repositorio:",
        "resolve files externally, then action+Shift+C" => "resuelve archivos externamente, luego action+Shift+C",
        "action" => "acción",
        "normal" => "normal",
        "Unsupported" => "No soportado",
        "Abort operation" => "Abortar operación",
        "Add remote" => "Añadir remoto",
        "Apply theme" => "Aplicar tema",
        "Apply language" => "Aplicar idioma",
        "Back" => "Atrás",
        "Back to graph" => "Volver al grafo",
        "Checkout" => "Checkout",
        "Checkout branch" => "Checkout de rama",
        "Cherry-pick" => "Cherry-pick",
        "Commit" => "Commit",
        "Continue operation" => "Continuar operación",
        "Create branch" => "Crear rama",
        "Create branch here" => "Crear rama aquí",
        "Create tag" => "Crear etiqueta",
        "Create worktree" => "Crear worktree",
        "Delete branch" => "Eliminar rama",
        "Delete remote" => "Eliminar remoto",
        "Delete tag" => "Eliminar etiqueta",
        "Discard file changes" => "Descartar cambios del archivo",
        "Drop stash" => "Descartar stash",
        "Edit fetch URL" => "Editar URL de fetch",
        "Edit push URL" => "Editar URL de push",
        "Exit" => "Salir",
        "Fetch" => "Fetch",
        "Find" => "Buscar",
        "Find file" => "Buscar archivo",
        "Hard reset" => "Hard reset",
        "Lock worktree" => "Bloquear worktree",
        "Merge" => "Merge",
        "Mixed reset" => "Mixed reset",
        "Move down" => "Mover abajo",
        "Move up" => "Mover arriba",
        "Open commit" => "Abrir commit",
        "Open file" => "Abrir archivo",
        "Open repository" => "Abrir repositorio",
        "Open stash commit" => "Abrir commit del stash",
        "Open submodule" => "Abrir submódulo",
        "Open worktree" => "Abrir worktree",
        "Pop stash" => "Aplicar stash",
        "Push" => "Push",
        "Push tags" => "Push de etiquetas",
        "Rebase" => "Rebase",
        "Rebind shortcut" => "Reasignar atajo",
        "Reload" => "Recargar",
        "Remove" => "Eliminar",
        "Remove worktree" => "Eliminar worktree",
        "Rename branch" => "Renombrar rama",
        "Rename remote" => "Renombrar remoto",
        "Return to parent repository" => "Volver al repositorio padre",
        "Revert" => "Revertir",
        "Set as default" => "Establecer como predeterminado",
        "Settings" => "Configuración",
        "Show details" => "Mostrar detalles",
        "Show files/status" => "Mostrar archivos/estado",
        "Show full diff" => "Mostrar diff completo",
        "Show hunk rows" => "Mostrar bloques",
        "Show split diff" => "Mostrar diff dividido",
        "Show unified diff" => "Mostrar diff unificado",
        "Solo branch" => "Aislar rama",
        "Splash screen" => "Pantalla inicial",
        "Stage all" => "Preparar todo",
        "Stage file" => "Preparar archivo",
        "Stage submodule" => "Preparar submódulo",
        "Stash changes" => "Guardar cambios en stash",
        "Sync URL" => "Sincronizar URL",
        "Toggle branch" => "Alternar rama",
        "Unlock worktree" => "Desbloquear worktree",
        "Unstage all" => "Quitar todo del índice",
        "Unstage file" => "Quitar archivo del índice",
        "Unstage submodule" => "Quitar submódulo del índice",
        "Update/init submodule" => "Actualizar/iniciar submódulo",
        "choose" => "elegir",
        "confirm" => "confirmar",
        "move" => "mover",
        "ok" => "ok",
        "save" => "guardar",
        "submit" => "enviar",
        "switch field" => "cambiar campo",
        "key:" => "clave:",
        "passphrase" => "frase de contraseña",
        "password / token" => "contraseña / token",
        "user:" => "usuario:",
        "username" => "usuario",
        "current:" => "actual:",
        "delete selected remote?" => "¿eliminar remoto seleccionado?",
        "error" => "error",
        "enter" => "enter",
        "tab" => "tab",
        "name:" => "nombre:",
        "new:" => "nuevo:",
        "new: waiting for key" => "nuevo: esperando tecla",
        "path:" => "ruta:",
        "press key" => "pulsa una tecla",
        "Enter cherry-pick commit message" => "Introduce mensaje de commit para cherry-pick",
        "Enter new branch name" => "Introduce nuevo nombre de rama",
        "Enter commit message" => "Introduce mensaje de commit",
        "Enter new tag name" => "Introduce nuevo nombre de etiqueta",
        "Enter new worktree name" => "Introduce nuevo nombre de worktree",
        "Enter new worktree path" => "Introduce nueva ruta de worktree",
        "Search repository files" => "Buscar archivos del repositorio",
        "Enter commit SHA to search for" => "Introduce SHA de commit a buscar",
        "Enter graph lane limit" => "Introduce límite de carriles del grafo",
        "Enter lock reason" => "Introduce motivo del bloqueo",
        "Enter new remote name" => "Introduce nuevo nombre de remoto",
        "Enter new remote URL" => "Introduce nueva URL de remoto",
        "Enter remote push URL" => "Introduce URL de push del remoto",
        "Enter remote fetch URL" => "Introduce URL de fetch del remoto",
        "Enter renamed remote name" => "Introduce nombre renombrado del remoto",
        "Enter renamed branch name" => "Introduce nombre renombrado de la rama",
        "Enter revert commit message" => "Introduce mensaje de commit de revert",
        "remote" => "remoto",
        "remote:" => "remoto:",
        "remove selected worktree?" => "¿eliminar worktree seleccionado?",
        "select a branch to checkout" => "selecciona una rama para checkout",
        "select a branch to delete" => "selecciona una rama para eliminar",
        "select a branch to rename" => "selecciona una rama para renombrar",
        "select a branch to solo" => "selecciona una rama para aislar",
        "select a branch to toggle" => "selecciona una rama para alternar",
        "select a tag to delete" => "selecciona una etiqueta para eliminar",
        "select a worktree to open" => "selecciona un worktree para abrir",
        "select a worktree to remove" => "selecciona un worktree para eliminar",
        "set shortcut" => "definir atajo",
        " type to search" => " escribe para buscar",
        " no matches" => " sin coincidencias",
        "Delete remote branch" => "Eliminar rama remota",
        "Git network operation" => "Operación de red de Git",
        "local" => "local",
        "Update submodule" => "Actualizar submódulo",
        "aborted" => "abortado",
        "cherrypick" => "cherry-pick",
        "Cherry-pick aborted." => "Cherry-pick abortado.",
        "Cherry-pick commit" => "Commit de cherry-pick",
        "Cherry-pick completed." => "Cherry-pick completado.",
        "Cherry-pick stopped because conflicts need to be resolved." => "Cherry-pick se detuvo porque hay conflictos por resolver.",
        "complete" => "completo",
        "conflict" => "conflicto",
        "merge" => "merge",
        "Merge already up to date." => "Merge ya está actualizado.",
        "Merge aborted." => "Merge abortado.",
        "Merge completed." => "Merge completado.",
        "Merge stopped because conflicts need to be resolved." => "Merge se detuvo porque hay conflictos por resolver.",
        "Merge fast-forwarded." => "Merge con avance rápido completado.",
        "rebase" => "rebase",
        "Rebase aborted." => "Rebase abortado.",
        "Rebase stopped because conflicts need to be resolved." => "Rebase se detuvo porque hay conflictos por resolver.",
        "revert" => "revert",
        "Revert aborted." => "Revert abortado.",
        "Revert commit" => "Commit de revert",
        "Revert completed." => "Revert completado.",
        "Revert stopped because conflicts need to be resolved." => "Revert se detuvo porque hay conflictos por resolver.",
        "resolve conflicts in your editor, then action+Shift+C" => "resuelve conflictos en tu editor, luego action+Shift+C",
        " actions:" => " acciones:",
        " active custom:" => " personalizado activo:",
        " active custom symbols:" => " símbolos personalizados activos:",
        "auth" => "auth",
        " authorization:" => " autorización:",
        "branches" => "ramas",
        "committer date/time" => "fecha/hora del committer",
        "committers" => "committers",
        " credentials:" => " credenciales:",
        " default remote:" => " remoto predeterminado:",
        "display" => "visualización",
        " email:" => " email:",
        "(enter)" => "(enter)",
        "general" => "general",
        " graph metadata:" => " metadatos del grafo:",
        " graph lane limit:" => " límite de carriles del grafo:",
        "graph reflog commits" => "commits de reflog en grafo",
        " https:" => " https:",
        "username/password or token prompt " => "solicitud de usuario/contraseña o token ",
        "inspector" => "inspector",
        " keymap:" => " mapa de teclas:",
        " language:" => " idioma:",
        " layout:" => " disposición:",
        " name:" => " nombre:",
        " pane visibility:" => " visibilidad de paneles:",
        " performance:" => " rendimiento:",
        "paths" => "rutas",
        " paths:" => " rutas:",
        " recent file:" => " archivo reciente:",
        " recent repositories:" => " repositorios recientes:",
        "reflog" => "reflog",
        "refs" => "refs",
        " remote error:" => " error de remoto:",
        " remotes:" => " remotos:",
        "select remote to manage | + add remote to create " => "selecciona remoto para gestionar | + añadir remoto para crear ",
        "repo" => "repo",
        "reset layout" => "restablecer disposición",
        " secrets:" => " secretos:",
        "session only " => "solo sesión ",
        "settings" => "configuración",
        "SHAs" => "SHAs",
        "shortcuts" => "atajos",
        " shortcuts / action mode:" => " atajos / modo acción:",
        " shortcuts / normal mode:" => " atajos / modo normal:",
        " ssh fallback:" => " respaldo ssh:",
        "key passphrase prompt " => "solicitud de frase de clave ",
        "ssh-agent when available " => "ssh-agent cuando esté disponible ",
        "stashes" => "stashes",
        "status" => "estado",
        "submodules" => "submódulos",
        " symbols:" => " símbolos:",
        "tags" => "etiquetas",
        " theme:" => " tema:",
        " themes:" => " temas:",
        " symbol theme:" => " tema de símbolos:",
        " symbol themes:" => " temas de símbolos:",
        " version:" => " versión:",
        "worktrees" => "worktrees",
        " + add remote" => " + añadir remoto",
        "fetch:" => "fetch:",
        "push:" => "push:",
        "actions:" => "acciones:",
        "loading..." => "cargando...",
        "made with ♡" => "hecho con ♡",
        "move down" => "mover abajo",
        "move up" => "mover arriba",
        "! not a valid git repository !" => "! no es un repositorio Git válido !",
        "recent repositories:" => "repositorios recientes:",
        "remove" => "eliminar",
        "detached" => "desacoplado",
        "detached head:" => "HEAD desacoplado:",
        "graph" => "grafo",
        "modal" => "modal",
        "no head (no commits yet)" => "sin HEAD (aún sin commits)",
        "staged" => "preparado",
        "stash" => "stash",
        "unstaged" => "no preparado",
        "viewer" => "visor",
        "modified" => "modificado",
        "new commits" => "commits nuevos",
        "untracked" => "sin seguimiento",
        "Widen scope" => "Ampliar alcance",
        "Narrow scope" => "Reducir alcance",
        "Focus next pane" => "Enfocar siguiente panel",
        "Focus previous pane" => "Enfocar panel anterior",
        "Focus pane left" => "Enfocar panel izquierdo",
        "Focus pane down" => "Enfocar panel inferior",
        "Focus pane up" => "Enfocar panel superior",
        "Focus pane right" => "Enfocar panel derecho",
        "Select" => "Seleccionar",
        "Minimize" => "Minimizar",
        "Reset layout" => "Restablecer disposición",
        "Shrink graph lane limit" => "Reducir límite de carriles del grafo",
        "Grow graph lane limit" => "Aumentar límite de carriles del grafo",
        "Resize pane left" => "Redimensionar panel a la izquierda",
        "Resize pane down" => "Redimensionar panel hacia abajo",
        "Resize pane up" => "Redimensionar panel hacia arriba",
        "Resize pane right" => "Redimensionar panel a la derecha",
        "Toggle zen mode" => "Alternar modo zen",
        "Toggle branches" => "Alternar ramas",
        "Toggle tags" => "Alternar etiquetas",
        "Toggle stashes" => "Alternar stashes",
        "Toggle reflogs" => "Alternar reflogs",
        "Toggle graph reflogs" => "Alternar reflogs del grafo",
        "Toggle graph dates" => "Alternar fechas del grafo",
        "Toggle graph committers" => "Alternar committers del grafo",
        "Toggle graph refs" => "Alternar refs del grafo",
        "Toggle worktrees" => "Alternar worktrees",
        "Toggle submodules" => "Alternar submódulos",
        "Toggle search" => "Alternar búsqueda",
        "Toggle status" => "Alternar estado",
        "Toggle inspector" => "Alternar inspector",
        "Toggle SHAs" => "Alternar SHAs",
        "Toggle help" => "Alternar ayuda",
        "Action mode" => "Modo acción",
        "Remove recent repository" => "Eliminar repositorio reciente",
        "Move recent repository up" => "Mover repositorio reciente arriba",
        "Move recent repository down" => "Mover repositorio reciente abajo",
        "Scroll page up" => "Desplazar página arriba",
        "Scroll page down" => "Desplazar página abajo",
        "Scroll half page up" => "Desplazar media página arriba",
        "Scroll half page down" => "Desplazar media página abajo",
        "Scroll up" => "Desplazar arriba",
        "Scroll down" => "Desplazar abajo",
        "Scroll up half" => "Desplazar media arriba",
        "Scroll down half" => "Desplazar media abajo",
        "Go to beginning" => "Ir al inicio",
        "Go to end" => "Ir al final",
        "Scroll up branch" => "Subir rama",
        "Scroll down branch" => "Bajar rama",
        "Scroll up commit" => "Subir commit",
        "Scroll down commit" => "Bajar commit",
        "Toggle hunk mode" => "Alternar modo de bloques",
        "Toggle split diff mode" => "Alternar diff dividido",
        "Fetch all" => "Fetch de todo",
        "Toggle worktree lock" => "Alternar bloqueo de worktree",
        "Reload all branches" => "Recargar todas las ramas",
        _ => es_extra(en),
    }
}

fn fr(en: &'static str) -> &'static str {
    match en {
        "default" => "par défaut",
        "loading" => "chargement",
        "none" => "aucun",
        "no head" => "pas de HEAD",
        "not initialized" => "non initialisé",
        "working..." => "travail en cours...",
        "no body" => "pas de corps",
        "no branches" => "aucune branche",
        "no commits" => "aucun commit",
        "no HEAD reflog" => "aucun reflog HEAD",
        "no message" => "aucun message",
        "no recent repositories" => "aucun dépôt récent",
        "no remotes" => "aucun distant",
        "no staged changes" => "aucun changement indexé",
        "no stashes" => "aucun stash",
        "no submodules" => "aucun sous-module",
        "no summary" => "aucun résumé",
        "no tags" => "aucun tag",
        "no unstaged changes" => "aucun changement non indexé",
        "no worktrees" => "aucun worktree",
        "search" => "rechercher",
        "Add remote failed" => "Échec de l'ajout du distant",
        "Checkout failed" => "Échec du checkout",
        "Cherry-pick failed" => "Échec du cherry-pick",
        "Commit failed" => "Échec du commit",
        "Create branch failed" => "Échec de la création de branche",
        "Create tag failed" => "Échec de la création du tag",
        "Create worktree failed" => "Échec de la création du worktree",
        "Delete branch failed" => "Échec de la suppression de branche",
        "Delete remote failed" => "Échec de la suppression du distant",
        "Delete tag failed" => "Échec de la suppression du tag",
        "Drop stash failed" => "Échec de la suppression du stash",
        "Edit remote failed" => "Échec de la modification du distant",
        "Couldn't get the file diff" => "Impossible d'obtenir le diff du fichier",
        "Git operation failed: no repository is open" => "Échec de l'opération Git : aucun dépôt ouvert",
        "Hard reset failed" => "Échec du hard reset",
        "Lock worktree failed" => "Échec du verrouillage du worktree",
        "Merge failed" => "Échec du merge",
        "Mixed reset failed" => "Échec du mixed reset",
        "Open repository failed" => "Échec de l'ouverture du dépôt",
        "Pop stash failed" => "Échec de l'application du stash",
        "Rebase failed" => "Échec du rebase",
        "Remove worktree failed" => "Échec de la suppression du worktree",
        "Rename branch failed" => "Échec du renommage de branche",
        "Rename remote failed" => "Échec du renommage du distant",
        "Reset file failed" => "Échec de la réinitialisation du fichier",
        "Revert failed" => "Échec du revert",
        "Save keymap failed" => "Échec de l'enregistrement du clavier",
        "Set default remote failed" => "Échec de la définition du distant par défaut",
        "Stage all failed" => "Échec de l'indexation de tout",
        "Stage file failed" => "Échec de l'indexation du fichier",
        "Stage submodule failed" => "Échec de l'indexation du sous-module",
        "Stash failed" => "Échec du stash",
        "Sync submodule failed" => "Échec de la synchronisation du sous-module",
        "Unstage all failed" => "Échec du désindexage de tout",
        "Unstage file failed" => "Échec du désindexage du fichier",
        "Unstage submodule failed" => "Échec du désindexage du sous-module",
        "Unlock worktree failed" => "Échec du déverrouillage du worktree",
        "authored by:" => "auteur :",
        "commit sha:" => "sha du commit :",
        "committed by:" => "commité par :",
        "conflicted files:" => "fichiers en conflit :",
        "featured branches:" => "branches mises en avant :",
        "head reflog:" => "reflog HEAD :",
        "message body:" => "corps du message :",
        "message summary:" => "résumé du message :",
        "next action:" => "action suivante :",
        "operation conflicts" => "conflits d'opération",
        "parent shas:" => "shas parents :",
        "repository state:" => "état du dépôt :",
        "resolve files externally, then action+Shift+C" => "résolvez les fichiers hors de l'app, puis action+Shift+C",
        "action" => "action",
        "normal" => "normal",
        "Unsupported" => "Non pris en charge",
        "Abort operation" => "Abandonner l'opération",
        "Add remote" => "Ajouter un distant",
        "Apply theme" => "Appliquer le thème",
        "Apply language" => "Appliquer la langue",
        "Back" => "Retour",
        "Back to graph" => "Retour au graphe",
        "Checkout branch" => "Checkout de branche",
        "Continue operation" => "Continuer l'opération",
        "Create branch" => "Créer une branche",
        "Create branch here" => "Créer une branche ici",
        "Create tag" => "Créer un tag",
        "Create worktree" => "Créer un worktree",
        "Delete branch" => "Supprimer la branche",
        "Delete remote" => "Supprimer le distant",
        "Delete tag" => "Supprimer le tag",
        "Discard file changes" => "Annuler les changements du fichier",
        "Edit fetch URL" => "Modifier l'URL de fetch",
        "Edit push URL" => "Modifier l'URL de push",
        "Exit" => "Quitter",
        "Find" => "Rechercher",
        "Find file" => "Rechercher un fichier",
        "Move down" => "Descendre",
        "Move up" => "Monter",
        "Open commit" => "Ouvrir le commit",
        "Open file" => "Ouvrir le fichier",
        "Open repository" => "Ouvrir le dépôt",
        "Open stash commit" => "Ouvrir le commit du stash",
        "Open submodule" => "Ouvrir le sous-module",
        "Open worktree" => "Ouvrir le worktree",
        "Rebind shortcut" => "Réassigner le raccourci",
        "Reload" => "Recharger",
        "Remove" => "Supprimer",
        "Remove worktree" => "Supprimer le worktree",
        "Rename branch" => "Renommer la branche",
        "Rename remote" => "Renommer le distant",
        "Return to parent repository" => "Retour au dépôt parent",
        "Set as default" => "Définir par défaut",
        "Settings" => "Paramètres",
        "Show details" => "Afficher les détails",
        "Show files/status" => "Afficher fichiers/état",
        "Show full diff" => "Afficher le diff complet",
        "Show hunk rows" => "Afficher les blocs",
        "Show split diff" => "Afficher le diff divisé",
        "Show unified diff" => "Afficher le diff unifié",
        "Solo branch" => "Isoler la branche",
        "Splash screen" => "Écran d'accueil",
        "Stage all" => "Indexer tout",
        "Stage file" => "Indexer le fichier",
        "Stage submodule" => "Indexer le sous-module",
        "Stash changes" => "Mettre les changements en stash",
        "Sync URL" => "Synchroniser l'URL",
        "Toggle branch" => "Basculer la branche",
        "Unlock worktree" => "Déverrouiller le worktree",
        "Unstage all" => "Désindexer tout",
        "Unstage file" => "Désindexer le fichier",
        "Unstage submodule" => "Désindexer le sous-module",
        "Update/init submodule" => "Mettre à jour/initialiser le sous-module",
        "choose" => "choisir",
        "confirm" => "confirmer",
        "move" => "déplacer",
        "save" => "enregistrer",
        "submit" => "envoyer",
        "switch field" => "changer de champ",
        "key:" => "clé :",
        "passphrase" => "phrase secrète",
        "password / token" => "mot de passe / jeton",
        "user:" => "utilisateur :",
        "username" => "nom d'utilisateur",
        "current:" => "actuel :",
        "delete selected remote?" => "supprimer le distant sélectionné ?",
        "name:" => "nom :",
        "new:" => "nouveau :",
        "new: waiting for key" => "nouveau : en attente d'une touche",
        "path:" => "chemin :",
        "press key" => "appuyez sur une touche",
        "Enter commit message" => "Saisir le message de commit",
        "Search repository files" => "Rechercher des fichiers du dépôt",
        "remote:" => "distant :",
        "remove selected worktree?" => "supprimer le worktree sélectionné ?",
        "set shortcut" => "définir le raccourci",
        " type to search" => " tapez pour rechercher",
        " no matches" => " aucune correspondance",
        "Git network operation" => "Opération réseau Git",
        "local" => "local",
        "aborted" => "abandonné",
        "complete" => "terminé",
        "conflict" => "conflit",
        "Cherry-pick aborted." => "Cherry-pick abandonné.",
        "Cherry-pick completed." => "Cherry-pick terminé.",
        "Merge completed." => "Merge terminé.",
        "Rebase aborted." => "Rebase abandonné.",
        "Revert completed." => "Revert terminé.",
        " actions:" => " actions :",
        " active custom:" => " personnalisé actif :",
        " active custom symbols:" => " symboles personnalisés actifs :",
        " authorization:" => " autorisation :",
        "branches" => "branches",
        " credentials:" => " identifiants :",
        " default remote:" => " distant par défaut :",
        "display" => "affichage",
        " email:" => " e-mail :",
        "general" => "général",
        " graph lane limit:" => " limite de voies du graphe :",
        " graph metadata:" => " métadonnées du graphe :",
        "inspector" => "inspecteur",
        " keymap:" => " clavier :",
        " language:" => " langue :",
        " layout:" => " disposition :",
        " name:" => " nom :",
        " pane visibility:" => " visibilité des panneaux :",
        " performance:" => " performances :",
        "paths" => "chemins",
        " paths:" => " chemins :",
        " recent file:" => " fichier récent :",
        " recent repositories:" => " dépôts récents :",
        " remote error:" => " erreur distante :",
        " remotes:" => " distants :",
        "repo" => "dépôt",
        "reset layout" => "réinitialiser la disposition",
        " settings" => " paramètres",
        "settings" => "paramètres",
        "shortcuts" => "raccourcis",
        " status" => " état",
        "status" => "état",
        "submodules" => "sous-modules",
        " symbols:" => " symboles :",
        "tags" => "tags",
        " theme:" => " thème :",
        " themes:" => " thèmes :",
        " symbol theme:" => " thème de symboles :",
        " symbol themes:" => " thèmes de symboles :",
        " version:" => " version :",
        "worktrees" => "worktrees",
        " + add remote" => " + ajouter un distant",
        "loading..." => "chargement...",
        "made with ♡" => "fait avec ♡",
        "! not a valid git repository !" => "! dépôt Git non valide !",
        "recent repositories:" => "dépôts récents :",
        "remove" => "supprimer",
        "detached" => "détaché",
        "detached head:" => "HEAD détaché :",
        "graph" => "graphe",
        "no head (no commits yet)" => "pas de HEAD (aucun commit)",
        "staged" => "indexé",
        "unstaged" => "non indexé",
        "viewer" => "visionneuse",
        "modified" => "modifié",
        "new commits" => "nouveaux commits",
        "untracked" => "non suivi",
        "Widen scope" => "Élargir la portée",
        "Narrow scope" => "Réduire la portée",
        "Focus next pane" => "Focus panneau suivant",
        "Focus previous pane" => "Focus panneau précédent",
        "Select" => "Sélectionner",
        "Minimize" => "Réduire",
        "Reset layout" => "Réinitialiser la disposition",
        "Toggle zen mode" => "Basculer le mode zen",
        "Toggle branches" => "Basculer les branches",
        "Toggle tags" => "Basculer les tags",
        "Toggle stashes" => "Basculer les stashes",
        "Toggle search" => "Basculer la recherche",
        "Action mode" => "Mode action",
        "Scroll up" => "Défiler vers le haut",
        "Scroll down" => "Défiler vers le bas",
        "Go to beginning" => "Aller au début",
        "Go to end" => "Aller à la fin",
        _ => fr_extra(en),
    }
}

fn ru(en: &'static str) -> &'static str {
    match en {
        "default" => "по умолчанию",
        "loading" => "загрузка",
        "none" => "нет",
        "no head" => "нет HEAD",
        "not initialized" => "не инициализировано",
        "working..." => "работа...",
        "no body" => "нет тела",
        "no branches" => "нет веток",
        "no commits" => "нет коммитов",
        "no HEAD reflog" => "нет reflog HEAD",
        "no message" => "нет сообщения",
        "no recent repositories" => "нет недавних репозиториев",
        "no remotes" => "нет удалённых",
        "no staged changes" => "нет индексированных изменений",
        "no stashes" => "нет stash",
        "no submodules" => "нет подмодулей",
        "no summary" => "нет сводки",
        "no tags" => "нет тегов",
        "no unstaged changes" => "нет неиндексированных изменений",
        "no worktrees" => "нет worktree",
        "search" => "поиск",
        "Add remote failed" => "Не удалось добавить удалённый",
        "Checkout failed" => "Checkout не удался",
        "Cherry-pick failed" => "Cherry-pick не удался",
        "Commit failed" => "Commit не удался",
        "Create branch failed" => "Не удалось создать ветку",
        "Create tag failed" => "Не удалось создать тег",
        "Create worktree failed" => "Не удалось создать worktree",
        "Delete branch failed" => "Не удалось удалить ветку",
        "Delete remote failed" => "Не удалось удалить удалённый",
        "Delete tag failed" => "Не удалось удалить тег",
        "Couldn't get the file diff" => "Не удалось получить diff файла",
        "Git operation failed: no repository is open" => "Операция Git не удалась: репозиторий не открыт",
        "Open repository failed" => "Не удалось открыть репозиторий",
        "authored by:" => "автор:",
        "commit sha:" => "sha коммита:",
        "committed by:" => "закоммитил:",
        "conflicted files:" => "конфликтующие файлы:",
        "message body:" => "тело сообщения:",
        "message summary:" => "сводка сообщения:",
        "next action:" => "следующее действие:",
        "repository state:" => "состояние репозитория:",
        "action" => "действие",
        "normal" => "обычный",
        "Abort operation" => "Прервать операцию",
        "Add remote" => "Добавить удалённый",
        "Apply theme" => "Применить тему",
        "Apply language" => "Применить язык",
        "Back" => "Назад",
        "Back to graph" => "Назад к графу",
        "Checkout branch" => "Checkout ветки",
        "Continue operation" => "Продолжить операцию",
        "Create branch" => "Создать ветку",
        "Create tag" => "Создать тег",
        "Delete branch" => "Удалить ветку",
        "Delete remote" => "Удалить удалённый",
        "Delete tag" => "Удалить тег",
        "Exit" => "Выход",
        "Find" => "Найти",
        "Find file" => "Найти файл",
        "Move down" => "Вниз",
        "Move up" => "Вверх",
        "Open commit" => "Открыть коммит",
        "Open file" => "Открыть файл",
        "Open repository" => "Открыть репозиторий",
        "Rebind shortcut" => "Переназначить сочетание",
        "Reload" => "Перезагрузить",
        "Remove" => "Удалить",
        "Rename branch" => "Переименовать ветку",
        "Rename remote" => "Переименовать удалённый",
        "Settings" => "Настройки",
        "Show details" => "Показать детали",
        "Stage all" => "Индексировать всё",
        "Stage file" => "Индексировать файл",
        "Unstage all" => "Убрать всё из индекса",
        "Unstage file" => "Убрать файл из индекса",
        "choose" => "выбрать",
        "confirm" => "подтвердить",
        "save" => "сохранить",
        "submit" => "отправить",
        "password / token" => "пароль / токен",
        "username" => "имя пользователя",
        "error" => "ошибка",
        "name:" => "имя:",
        "path:" => "путь:",
        "press key" => "нажмите клавишу",
        "Git network operation" => "Сетевая операция Git",
        "aborted" => "прервано",
        "complete" => "завершено",
        "conflict" => "конфликт",
        " actions:" => " действия:",
        "branches" => "ветки",
        " credentials:" => " учётные данные:",
        " default remote:" => " удалённый по умолчанию:",
        "display" => "экран",
        "general" => "общие",
        " graph lane limit:" => " лимит дорожек графа:",
        " language:" => " язык:",
        " pane visibility:" => " видимость панелей:",
        " performance:" => " производительность:",
        "paths" => "пути",
        " recent repositories:" => " недавние репозитории:",
        " remotes:" => " удалённые:",
        "repo" => "репо",
        "settings" => "настройки",
        "shortcuts" => "сочетания",
        "status" => "статус",
        "submodules" => "подмодули",
        "tags" => "теги",
        " themes:" => " темы:",
        " symbol themes:" => " темы символов:",
        " version:" => " версия:",
        "loading..." => "загрузка...",
        "made with ♡" => "сделано с ♡",
        "recent repositories:" => "недавние репозитории:",
        "remove" => "удалить",
        "detached" => "отсоединённый",
        "graph" => "граф",
        "staged" => "в индексе",
        "unstaged" => "не в индексе",
        "viewer" => "просмотр",
        "modified" => "изменено",
        "new commits" => "новые коммиты",
        "untracked" => "неотслеживаемые",
        "Select" => "Выбрать",
        "Scroll up" => "Прокрутить вверх",
        "Scroll down" => "Прокрутить вниз",
        "Go to beginning" => "К началу",
        "Go to end" => "К концу",
        _ => ru_extra(en),
    }
}

fn tr_tr(en: &'static str) -> &'static str {
    match en {
        "default" => "varsayılan",
        "loading" => "yükleniyor",
        "none" => "yok",
        "no head" => "HEAD yok",
        "not initialized" => "başlatılmadı",
        "working..." => "çalışıyor...",
        "no body" => "gövde yok",
        "no branches" => "dal yok",
        "no commits" => "commit yok",
        "no HEAD reflog" => "HEAD reflog yok",
        "no message" => "mesaj yok",
        "no recent repositories" => "son depo yok",
        "no remotes" => "remote yok",
        "no staged changes" => "stage edilmiş değişiklik yok",
        "no stashes" => "stash yok",
        "no submodules" => "alt modül yok",
        "no summary" => "özet yok",
        "no tags" => "etiket yok",
        "no unstaged changes" => "stage edilmemiş değişiklik yok",
        "no worktrees" => "worktree yok",
        "search" => "ara",
        "Add remote failed" => "Remote ekleme başarısız",
        "Checkout failed" => "Checkout başarısız",
        "Cherry-pick failed" => "Cherry-pick başarısız",
        "Commit failed" => "Commit başarısız",
        "Create branch failed" => "Dal oluşturma başarısız",
        "Create tag failed" => "Etiket oluşturma başarısız",
        "Create worktree failed" => "Worktree oluşturma başarısız",
        "Delete branch failed" => "Dal silme başarısız",
        "Delete remote failed" => "Remote silme başarısız",
        "Delete tag failed" => "Etiket silme başarısız",
        "Couldn't get the file diff" => "Dosya diff'i alınamadı",
        "Open repository failed" => "Depo açılamadı",
        "authored by:" => "yazar:",
        "commit sha:" => "commit sha:",
        "committed by:" => "commit yapan:",
        "conflicted files:" => "çakışan dosyalar:",
        "message body:" => "mesaj gövdesi:",
        "message summary:" => "mesaj özeti:",
        "next action:" => "sonraki eylem:",
        "repository state:" => "depo durumu:",
        "action" => "eylem",
        "normal" => "normal",
        "Abort operation" => "Operasyonu iptal et",
        "Add remote" => "Remote ekle",
        "Apply theme" => "Temayı uygula",
        "Apply language" => "Dili uygula",
        "Back" => "Geri",
        "Back to graph" => "Grafa dön",
        "Create branch" => "Dal oluştur",
        "Create tag" => "Etiket oluştur",
        "Delete branch" => "Dalı sil",
        "Delete remote" => "Remote sil",
        "Delete tag" => "Etiketi sil",
        "Exit" => "Çık",
        "Find" => "Bul",
        "Find file" => "Dosya bul",
        "Move down" => "Aşağı taşı",
        "Move up" => "Yukarı taşı",
        "Open commit" => "Commit aç",
        "Open file" => "Dosya aç",
        "Open repository" => "Depo aç",
        "Rebind shortcut" => "Kısayolu değiştir",
        "Reload" => "Yenile",
        "Remove" => "Kaldır",
        "Rename branch" => "Dalı yeniden adlandır",
        "Rename remote" => "Remote yeniden adlandır",
        "Settings" => "Ayarlar",
        "Show details" => "Ayrıntıları göster",
        "Stage all" => "Tümünü stage et",
        "Stage file" => "Dosyayı stage et",
        "Unstage all" => "Tümünü stage'den çıkar",
        "Unstage file" => "Dosyayı stage'den çıkar",
        "choose" => "seç",
        "confirm" => "onayla",
        "save" => "kaydet",
        "submit" => "gönder",
        "password / token" => "parola / token",
        "username" => "kullanıcı adı",
        "error" => "hata",
        "name:" => "ad:",
        "path:" => "yol:",
        "press key" => "tuşa bas",
        "Git network operation" => "Git ağ işlemi",
        "aborted" => "iptal edildi",
        "complete" => "tamamlandı",
        "conflict" => "çakışma",
        " actions:" => " eylemler:",
        "branches" => "dallar",
        " credentials:" => " kimlik bilgileri:",
        " default remote:" => " varsayılan remote:",
        "display" => "görüntü",
        "general" => "genel",
        " graph lane limit:" => " grafik şerit sınırı:",
        " language:" => " dil:",
        " pane visibility:" => " panel görünürlüğü:",
        " performance:" => " performans:",
        "paths" => "yollar",
        " recent repositories:" => " son depolar:",
        " remotes:" => " remote'lar:",
        "repo" => "depo",
        "settings" => "ayarlar",
        "shortcuts" => "kısayollar",
        "status" => "durum",
        "submodules" => "alt modüller",
        "tags" => "etiketler",
        " themes:" => " temalar:",
        " symbol themes:" => " sembol temaları:",
        " version:" => " sürüm:",
        "loading..." => "yükleniyor...",
        "made with ♡" => "♡ ile yapıldı",
        "recent repositories:" => "son depolar:",
        "remove" => "kaldır",
        "detached" => "ayrık",
        "graph" => "graf",
        "staged" => "stage'de",
        "unstaged" => "stage dışı",
        "viewer" => "görüntüleyici",
        "modified" => "değiştirildi",
        "new commits" => "yeni commitler",
        "untracked" => "izlenmeyen",
        "Select" => "Seç",
        "Scroll up" => "Yukarı kaydır",
        "Scroll down" => "Aşağı kaydır",
        "Go to beginning" => "Başa git",
        "Go to end" => "Sona git",
        _ => tr_extra(en),
    }
}

fn es_extra(en: &'static str) -> &'static str {
    match en {
        " settings" => " configuración",
        " status" => " estado",
        _ => en,
    }
}

fn fr_extra(en: &'static str) -> &'static str {
    match en {
        " https:" => " https :",
        " secrets:" => " secrets :",
        " shortcuts / action mode:" => " raccourcis / mode action :",
        " shortcuts / normal mode:" => " raccourcis / mode normal :",
        " ssh fallback:" => " secours ssh :",
        "(enter)" => "(entrée)",
        "Abort failed: no rebase, cherry-pick, revert, or merge in progress" => "Échec de l’abandon : aucun rebase, cherry-pick, revert ou merge en cours",
        "Add remote failed: remote name is invalid" => "Échec de l’ajout du distant : le nom du distant est invalide",
        "Checkout" => "Checkout",
        "Cherry-pick" => "Cherry-pick",
        "Cherry-pick commit" => "Commit de cherry-pick",
        "Cherry-pick failed: no commit is pending" => "Échec du cherry-pick : aucun commit en attente",
        "Cherry-pick failed: no commit message was provided" => "Échec du cherry-pick : aucun message de commit fourni",
        "Cherry-pick stopped because conflicts need to be resolved." => "Cherry-pick arrêté car des conflits doivent être résolus.",
        "Commit" => "Commit",
        "Continue failed: no rebase, cherry-pick, revert, or merge in progress" => "Échec de la continuation : aucun rebase, cherry-pick, revert ou merge en cours",
        "Create branch failed: no commit is selected" => "Échec de la création de branche : aucun commit sélectionné",
        "Create tag failed: no commit is selected" => "Échec de la création du tag : aucun commit sélectionné",
        "Create worktree failed: names cannot be empty or contain path separators" => "Échec de la création du worktree : les noms ne peuvent pas être vides ni contenir de séparateurs de chemin",
        "Create worktree failed: no commit is selected" => "Échec de la création du worktree : aucun commit sélectionné",
        "Create worktree failed: path cannot be empty" => "Échec de la création du worktree : le chemin ne peut pas être vide",
        "Delete branch failed: cannot delete the current branch" => "Échec de la suppression de branche : impossible de supprimer la branche actuelle",
        "Delete branch failed: remote branch name is invalid" => "Échec de la suppression de branche : le nom de branche distante est invalide",
        "Delete remote branch" => "Supprimer la branche distante",
        "Delete remote failed: no remote is pending" => "Échec de la suppression du distant : aucun distant en attente",
        "Drop stash" => "Supprimer le stash",
        "Edit remote failed: no remote is pending" => "Échec de la modification du distant : aucun distant en attente",
        "Enter cherry-pick commit message" => "Saisir le message de commit du cherry-pick",
        "Enter commit SHA to search for" => "Saisir le SHA de commit à rechercher",
        "Enter graph lane limit" => "Saisir la limite de voies du graphe",
        "Enter lock reason" => "Saisir la raison du verrouillage",
        "Enter new branch name" => "Saisir le nouveau nom de branche",
        "Enter new remote URL" => "Saisir la nouvelle URL du distant",
        "Enter new remote name" => "Saisir le nouveau nom du distant",
        "Enter new tag name" => "Saisir le nouveau nom de tag",
        "Enter new worktree name" => "Saisir le nouveau nom de worktree",
        "Enter new worktree path" => "Saisir le nouveau chemin de worktree",
        "Enter remote fetch URL" => "Saisir l’URL de fetch du distant",
        "Enter remote push URL" => "Saisir l’URL de push du distant",
        "Enter renamed branch name" => "Saisir le nouveau nom de la branche",
        "Enter renamed remote name" => "Saisir le nouveau nom du distant",
        "Enter revert commit message" => "Saisir le message de commit du revert",
        "Fetch" => "Fetch",
        "Fetch all" => "Tout fetcher",
        "File history failed: graph worker is unavailable" => "Échec de l’historique du fichier : le worker du graphe est indisponible",
        "Focus pane down" => "Activer le panneau inférieur",
        "Focus pane left" => "Activer le panneau gauche",
        "Focus pane right" => "Activer le panneau droit",
        "Focus pane up" => "Activer le panneau supérieur",
        "Git network operation failed: another network operation is already running" => "Échec de l’opération réseau Git : une autre opération réseau est déjà en cours",
        "Git network operation failed: worker thread panicked" => "Échec de l’opération réseau Git : le thread worker a paniqué",
        "Hard reset" => "Hard reset",
        "Lock worktree" => "Verrouiller le worktree",
        "Lock worktree failed: only valid linked worktrees can be locked" => "Échec du verrouillage du worktree : seuls les worktrees liés valides peuvent être verrouillés",
        "Merge" => "Merge",
        "Merge aborted." => "Merge abandonné.",
        "Merge already up to date." => "Merge déjà à jour.",
        "Merge fast-forwarded." => "Merge fast-forward effectué.",
        "Merge stopped because conflicts need to be resolved." => "Merge arrêté car des conflits doivent être résolus.",
        "Mixed reset" => "Mixed reset",
        "Move recent repository down" => "Descendre le dépôt récent",
        "Move recent repository up" => "Monter le dépôt récent",
        "Open submodule failed: submodule is not initialized. Run update/init first." => "Échec de l’ouverture du sous-module : le sous-module n’est pas initialisé. Lancez update/init d’abord.",
        "Open worktree failed: worktree path is invalid" => "Échec de l’ouverture du worktree : le chemin du worktree est invalide",
        "Pop stash" => "Appliquer le stash",
        "Push" => "Push",
        "Push failed: detached HEAD has no current branch" => "Échec du push : HEAD détaché n’a pas de branche actuelle",
        "Push tags" => "Pusher les tags",
        "Rebase" => "Rebase",
        "Rebase stopped because conflicts need to be resolved." => "Rebase arrêté car des conflits doivent être résolus.",
        "Reflog commit is hidden from the graph. Press 9 to show graph reflogs." => "Le commit du reflog est masqué dans le graphe. Appuyez sur 9 pour afficher les reflogs du graphe.",
        "Reload all branches" => "Recharger toutes les branches",
        "Remove recent repository" => "Supprimer le dépôt récent",
        "Remove worktree failed: cannot remove current, main, or locked worktrees" => "Échec de la suppression du worktree : impossible de supprimer les worktrees actuel, principal ou verrouillés",
        "Rename branch failed: no branch is pending" => "Échec du renommage de branche : aucune branche en attente",
        "Rename branch failed: only local branches can be renamed" => "Échec du renommage de branche : seules les branches locales peuvent être renommées",
        "Rename remote failed: no remote is pending" => "Échec du renommage du distant : aucun distant en attente",
        "Shrink graph lane limit" => "Réduire la limite de voies du graphe",
        "Grow graph lane limit" => "Augmenter la limite de voies du graphe",
        "Resize pane down" => "Redimensionner le panneau vers le bas",
        "Resize pane left" => "Redimensionner le panneau vers la gauche",
        "Resize pane right" => "Redimensionner le panneau vers la droite",
        "Resize pane up" => "Redimensionner le panneau vers le haut",
        "Revert" => "Revert",
        "Revert aborted." => "Revert abandonné.",
        "Revert commit" => "Commit de revert",
        "Revert failed: no commit is pending" => "Échec du revert : aucun commit en attente",
        "Revert failed: no commit message was provided" => "Échec du revert : aucun message de commit fourni",
        "Revert failed: reverting merge commits is not supported" => "Échec du revert : le revert des commits de merge n’est pas pris en charge",
        "Revert stopped because conflicts need to be resolved." => "Revert arrêté car des conflits doivent être résolus.",
        "SHAs" => "SHAs",
        "Scroll down branch" => "Branche suivante",
        "Scroll down commit" => "Commit suivant",
        "Scroll down half" => "Défiler d’une demi-page vers le bas",
        "Scroll half page down" => "Défiler d’une demi-page vers le bas",
        "Scroll half page up" => "Défiler d’une demi-page vers le haut",
        "Scroll page down" => "Page suivante",
        "Scroll page up" => "Page précédente",
        "Scroll up branch" => "Branche précédente",
        "Scroll up commit" => "Commit précédent",
        "Scroll up half" => "Défiler d’une demi-page vers le haut",
        "Stage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Échec de l’indexation du fichier : résolvez les conflits dans votre éditeur, puis continuez l’opération active"
        },
        "Toggle SHAs" => "Basculer les SHAs",
        "Toggle graph committers" => "Basculer les committers du graphe",
        "Toggle graph dates" => "Basculer les dates du graphe",
        "Toggle graph reflogs" => "Basculer les reflogs du graphe",
        "Toggle graph refs" => "Basculer les refs du graphe",
        "Toggle help" => "Basculer l’aide",
        "Toggle hunk mode" => "Basculer le mode blocs",
        "Toggle inspector" => "Basculer l’inspecteur",
        "Toggle reflogs" => "Basculer les reflogs",
        "Toggle split diff mode" => "Basculer le diff divisé",
        "Toggle status" => "Basculer l’état",
        "Toggle submodules" => "Basculer les sous-modules",
        "Toggle worktree lock" => "Basculer le verrouillage du worktree",
        "Toggle worktrees" => "Basculer les worktrees",
        "Unstage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Échec du désindexage du fichier : résolvez les conflits dans votre éditeur, puis continuez l’opération active"
        },
        "Update submodule" => "Mettre à jour le sous-module",
        "actions:" => "actions :",
        "auth" => "auth",
        "cherrypick" => "cherry-pick",
        "committer date/time" => "date/heure du committer",
        "committers" => "committers",
        "enter" => "entrée",
        "error" => "erreur",
        "fetch:" => "fetch :",
        "graph reflog commits" => "commits de reflog du graphe",
        "key passphrase prompt " => "invite de phrase secrète de clé ",
        "merge" => "merge",
        "modal" => "modale",
        "move down" => "descendre",
        "move up" => "monter",
        "ok" => "ok",
        "push:" => "push :",
        "rebase" => "rebase",
        "reflog" => "reflog",
        "refs" => "refs",
        "remote" => "distant",
        "resolve conflicts in your editor, then action+Shift+C" => "résolvez les conflits dans votre éditeur, puis action+Shift+C",
        "revert" => "revert",
        "select a branch to checkout" => "sélectionnez une branche à checkout",
        "select a branch to delete" => "sélectionnez une branche à supprimer",
        "select a branch to rename" => "sélectionnez une branche à renommer",
        "select a branch to solo" => "sélectionnez une branche à isoler",
        "select a branch to toggle" => "sélectionnez une branche à basculer",
        "select a tag to delete" => "sélectionnez un tag à supprimer",
        "select a worktree to open" => "sélectionnez un worktree à ouvrir",
        "select a worktree to remove" => "sélectionnez un worktree à supprimer",
        "select remote to manage | + add remote to create " => "sélectionnez un distant à gérer | + ajouter un distant pour créer ",
        "session only " => "session uniquement ",
        "ssh-agent when available " => "ssh-agent si disponible ",
        "stash" => "stash",
        "stashes" => "stashes",
        "tab" => "tabulation",
        "username/password or token prompt " => "invite nom d’utilisateur/mot de passe ou jeton ",
        _ => en,
    }
}

fn ru_extra(en: &'static str) -> &'static str {
    match en {
        " + add remote" => " + добавить удалённый",
        " active custom symbols:" => " активные пользовательские символы:",
        " active custom:" => " активная пользовательская:",
        " authorization:" => " авторизация:",
        " email:" => " e-mail:",
        " graph metadata:" => " метаданные графа:",
        " https:" => " https:",
        " keymap:" => " раскладка клавиш:",
        " layout:" => " макет:",
        " name:" => " имя:",
        " no matches" => " нет совпадений",
        " paths:" => " пути:",
        " recent file:" => " недавний файл:",
        " remote error:" => " ошибка удалённого:",
        " secrets:" => " секреты:",
        " settings" => " настройки",
        " shortcuts / action mode:" => " сочетания / режим действий:",
        " shortcuts / normal mode:" => " сочетания / обычный режим:",
        " ssh fallback:" => " резервный ssh:",
        " status" => " статус",
        " symbol theme:" => " тема символов:",
        " symbols:" => " символы:",
        " theme:" => " тема:",
        " type to search" => " введите для поиска",
        "! not a valid git repository !" => "! недопустимый Git-репозиторий !",
        "(enter)" => "(enter)",
        "Abort failed: no rebase, cherry-pick, revert, or merge in progress" => "Не удалось прервать: нет rebase, cherry-pick, revert или merge в процессе",
        "Action mode" => "Режим действий",
        "Add remote failed: remote name is invalid" => "Не удалось добавить удалённый: имя удалённого недопустимо",
        "Checkout" => "Checkout",
        "Cherry-pick" => "Cherry-pick",
        "Cherry-pick aborted." => "Cherry-pick прерван.",
        "Cherry-pick commit" => "Commit cherry-pick",
        "Cherry-pick completed." => "Cherry-pick завершён.",
        "Cherry-pick failed: no commit is pending" => "Cherry-pick не удался: нет ожидающего коммита",
        "Cherry-pick failed: no commit message was provided" => "Cherry-pick не удался: сообщение commit не указано",
        "Cherry-pick stopped because conflicts need to be resolved." => "Cherry-pick остановлен: нужно разрешить конфликты.",
        "Commit" => "Commit",
        "Continue failed: no rebase, cherry-pick, revert, or merge in progress" => "Не удалось продолжить: нет rebase, cherry-pick, revert или merge в процессе",
        "Create branch failed: no commit is selected" => "Не удалось создать ветку: commit не выбран",
        "Create branch here" => "Создать ветку здесь",
        "Create tag failed: no commit is selected" => "Не удалось создать тег: commit не выбран",
        "Create worktree" => "Создать worktree",
        "Create worktree failed: names cannot be empty or contain path separators" => "Не удалось создать worktree: имена не могут быть пустыми или содержать разделители пути",
        "Create worktree failed: no commit is selected" => "Не удалось создать worktree: commit не выбран",
        "Create worktree failed: path cannot be empty" => "Не удалось создать worktree: путь не может быть пустым",
        "Delete branch failed: cannot delete the current branch" => "Не удалось удалить ветку: нельзя удалить текущую ветку",
        "Delete branch failed: remote branch name is invalid" => "Не удалось удалить ветку: имя удалённой ветки недопустимо",
        "Delete remote branch" => "Удалить удалённую ветку",
        "Delete remote failed: no remote is pending" => "Не удалось удалить удалённый: нет ожидающего удалённого",
        "Discard file changes" => "Отбросить изменения файла",
        "Drop stash" => "Удалить stash",
        "Drop stash failed" => "Не удалось удалить stash",
        "Edit fetch URL" => "Изменить URL fetch",
        "Edit push URL" => "Изменить URL push",
        "Edit remote failed" => "Не удалось изменить удалённый",
        "Edit remote failed: no remote is pending" => "Не удалось изменить удалённый: нет ожидающего удалённого",
        "Enter cherry-pick commit message" => "Введите сообщение commit для cherry-pick",
        "Enter commit SHA to search for" => "Введите SHA commit для поиска",
        "Enter graph lane limit" => "Введите лимит дорожек графа",
        "Enter commit message" => "Введите сообщение commit",
        "Enter lock reason" => "Введите причину блокировки",
        "Enter new branch name" => "Введите новое имя ветки",
        "Enter new remote URL" => "Введите новый URL удалённого",
        "Enter new remote name" => "Введите новое имя удалённого",
        "Enter new tag name" => "Введите новое имя тега",
        "Enter new worktree name" => "Введите новое имя worktree",
        "Enter new worktree path" => "Введите новый путь worktree",
        "Enter remote fetch URL" => "Введите URL fetch удалённого",
        "Enter remote push URL" => "Введите URL push удалённого",
        "Enter renamed branch name" => "Введите новое имя ветки",
        "Enter renamed remote name" => "Введите новое имя удалённого",
        "Enter revert commit message" => "Введите сообщение commit для revert",
        "Fetch" => "Fetch",
        "Fetch all" => "Fetch всех",
        "File history failed: graph worker is unavailable" => "История файла недоступна: worker графа недоступен",
        "Focus next pane" => "Фокус на следующую панель",
        "Focus pane down" => "Фокус на нижнюю панель",
        "Focus pane left" => "Фокус на левую панель",
        "Focus pane right" => "Фокус на правую панель",
        "Focus pane up" => "Фокус на верхнюю панель",
        "Focus previous pane" => "Фокус на предыдущую панель",
        "Git network operation failed: another network operation is already running" => "Сетевая операция Git не удалась: другая сетевая операция уже выполняется",
        "Git network operation failed: worker thread panicked" => "Сетевая операция Git не удалась: worker thread panicked",
        "Hard reset" => "Hard reset",
        "Hard reset failed" => "Hard reset не удался",
        "Lock worktree" => "Заблокировать worktree",
        "Lock worktree failed" => "Не удалось заблокировать worktree",
        "Lock worktree failed: only valid linked worktrees can be locked" => "Не удалось заблокировать worktree: можно блокировать только допустимые связанные worktree",
        "Merge" => "Merge",
        "Merge aborted." => "Merge прерван.",
        "Merge already up to date." => "Merge уже актуален.",
        "Merge completed." => "Merge завершён.",
        "Merge failed" => "Merge не удался",
        "Merge fast-forwarded." => "Merge выполнен fast-forward.",
        "Merge stopped because conflicts need to be resolved." => "Merge остановлен: нужно разрешить конфликты.",
        "Minimize" => "Свернуть",
        "Mixed reset" => "Mixed reset",
        "Mixed reset failed" => "Mixed reset не удался",
        "Move recent repository down" => "Переместить недавний репозиторий вниз",
        "Move recent repository up" => "Переместить недавний репозиторий вверх",
        "Narrow scope" => "Сузить область",
        "Open stash commit" => "Открыть commit stash",
        "Open submodule" => "Открыть подмодуль",
        "Open submodule failed: submodule is not initialized. Run update/init first." => "Не удалось открыть подмодуль: подмодуль не инициализирован. Сначала выполните update/init.",
        "Open worktree" => "Открыть worktree",
        "Open worktree failed: worktree path is invalid" => "Не удалось открыть worktree: путь worktree недопустим",
        "Pop stash" => "Применить stash",
        "Pop stash failed" => "Не удалось применить stash",
        "Push" => "Push",
        "Push failed: detached HEAD has no current branch" => "Push не удался: detached HEAD не имеет текущей ветки",
        "Push tags" => "Push тегов",
        "Rebase" => "Rebase",
        "Rebase aborted." => "Rebase прерван.",
        "Rebase failed" => "Rebase не удался",
        "Rebase stopped because conflicts need to be resolved." => "Rebase остановлен: нужно разрешить конфликты.",
        "Reflog commit is hidden from the graph. Press 9 to show graph reflogs." => "Commit reflog скрыт из графа. Нажмите 9, чтобы показать reflog графа.",
        "Reload all branches" => "Перезагрузить все ветки",
        "Remove recent repository" => "Удалить недавний репозиторий",
        "Remove worktree" => "Удалить worktree",
        "Remove worktree failed" => "Не удалось удалить worktree",
        "Remove worktree failed: cannot remove current, main, or locked worktrees" => "Не удалось удалить worktree: нельзя удалить текущий, основной или заблокированный worktree",
        "Rename branch failed" => "Не удалось переименовать ветку",
        "Rename branch failed: no branch is pending" => "Не удалось переименовать ветку: нет ожидающей ветки",
        "Rename branch failed: only local branches can be renamed" => "Не удалось переименовать ветку: можно переименовывать только локальные ветки",
        "Rename remote failed" => "Не удалось переименовать удалённый",
        "Rename remote failed: no remote is pending" => "Не удалось переименовать удалённый: нет ожидающего удалённого",
        "Reset file failed" => "Не удалось сбросить файл",
        "Reset layout" => "Сбросить макет",
        "Resize pane down" => "Изменить размер панели вниз",
        "Resize pane left" => "Изменить размер панели влево",
        "Resize pane right" => "Изменить размер панели вправо",
        "Resize pane up" => "Изменить размер панели вверх",
        "Return to parent repository" => "Вернуться к родительскому репозиторию",
        "Revert" => "Revert",
        "Revert aborted." => "Revert прерван.",
        "Revert commit" => "Commit revert",
        "Revert completed." => "Revert завершён.",
        "Revert failed" => "Revert не удался",
        "Revert failed: no commit is pending" => "Revert не удался: нет ожидающего commit",
        "Revert failed: no commit message was provided" => "Revert не удался: сообщение commit не указано",
        "Revert failed: reverting merge commits is not supported" => "Revert не удался: revert merge-коммитов не поддерживается",
        "Revert stopped because conflicts need to be resolved." => "Revert остановлен: нужно разрешить конфликты.",
        "SHAs" => "SHA",
        "Save keymap failed" => "Не удалось сохранить раскладку клавиш",
        "Scroll down branch" => "Следующая ветка",
        "Scroll down commit" => "Следующий commit",
        "Scroll down half" => "Прокрутить на половину вниз",
        "Scroll half page down" => "Прокрутить полстраницы вниз",
        "Scroll half page up" => "Прокрутить полстраницы вверх",
        "Scroll page down" => "Прокрутить страницу вниз",
        "Scroll page up" => "Прокрутить страницу вверх",
        "Scroll up branch" => "Предыдущая ветка",
        "Scroll up commit" => "Предыдущий commit",
        "Scroll up half" => "Прокрутить на половину вверх",
        "Search repository files" => "Искать файлы репозитория",
        "Set as default" => "Сделать по умолчанию",
        "Set default remote failed" => "Не удалось задать удалённый по умолчанию",
        "Show files/status" => "Показать файлы/статус",
        "Show full diff" => "Показать полный diff",
        "Show hunk rows" => "Показать строки hunks",
        "Show split diff" => "Показать разделённый diff",
        "Show unified diff" => "Показать unified diff",
        "Solo branch" => "Изолировать ветку",
        "Splash screen" => "Стартовый экран",
        "Stage all failed" => "Не удалось индексировать всё",
        "Stage file failed" => "Не удалось индексировать файл",
        "Stage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Не удалось индексировать файл: разрешите конфликты в редакторе, затем продолжите активную операцию"
        },
        "Stage submodule" => "Индексировать подмодуль",
        "Stage submodule failed" => "Не удалось индексировать подмодуль",
        "Stash changes" => "Сохранить изменения в stash",
        "Stash failed" => "Stash не удался",
        "Sync URL" => "Синхронизировать URL",
        "Sync submodule failed" => "Не удалось синхронизировать подмодуль",
        "Shrink graph lane limit" => "Уменьшить лимит дорожек графа",
        "Grow graph lane limit" => "Увеличить лимит дорожек графа",
        "Toggle SHAs" => "Переключить SHA",
        "Toggle branch" => "Переключить ветку",
        "Toggle branches" => "Переключить ветки",
        "Toggle graph committers" => "Переключить committers графа",
        "Toggle graph dates" => "Переключить даты графа",
        "Toggle graph reflogs" => "Переключить reflog графа",
        "Toggle graph refs" => "Переключить refs графа",
        "Toggle help" => "Переключить справку",
        "Toggle hunk mode" => "Переключить режим hunks",
        "Toggle inspector" => "Переключить инспектор",
        "Toggle reflogs" => "Переключить reflog",
        "Toggle search" => "Переключить поиск",
        "Toggle split diff mode" => "Переключить разделённый diff",
        "Toggle stashes" => "Переключить stash",
        "Toggle status" => "Переключить статус",
        "Toggle submodules" => "Переключить подмодули",
        "Toggle tags" => "Переключить теги",
        "Toggle worktree lock" => "Переключить блокировку worktree",
        "Toggle worktrees" => "Переключить worktree",
        "Toggle zen mode" => "Переключить zen-режим",
        "Unlock worktree" => "Разблокировать worktree",
        "Unlock worktree failed" => "Не удалось разблокировать worktree",
        "Unstage all failed" => "Не удалось убрать всё из индекса",
        "Unstage file failed" => "Не удалось убрать файл из индекса",
        "Unstage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Не удалось убрать файл из индекса: разрешите конфликты в редакторе, затем продолжите активную операцию"
        },
        "Unstage submodule" => "Убрать подмодуль из индекса",
        "Unstage submodule failed" => "Не удалось убрать подмодуль из индекса",
        "Unsupported" => "Не поддерживается",
        "Update submodule" => "Обновить подмодуль",
        "Update/init submodule" => "Обновить/инициализировать подмодуль",
        "Widen scope" => "Расширить область",
        "actions:" => "действия:",
        "auth" => "auth",
        "cherrypick" => "cherry-pick",
        "committer date/time" => "дата/время коммиттера",
        "committers" => "коммиттеры",
        "current:" => "текущий:",
        "delete selected remote?" => "удалить выбранный удалённый?",
        "detached head:" => "detached HEAD:",
        "enter" => "enter",
        "featured branches:" => "избранные ветки:",
        "fetch:" => "fetch:",
        "graph reflog commits" => "commits reflog в графе",
        "head reflog:" => "reflog HEAD:",
        "inspector" => "инспектор",
        "key passphrase prompt " => "запрос passphrase ключа ",
        "key:" => "ключ:",
        "local" => "локальный",
        "merge" => "merge",
        "modal" => "модальное окно",
        "move" => "переместить",
        "move down" => "переместить вниз",
        "move up" => "переместить вверх",
        "new:" => "новый:",
        "new: waiting for key" => "новый: ожидание клавиши",
        "no head (no commits yet)" => "нет HEAD (пока нет commit)",
        "ok" => "ok",
        "operation conflicts" => "конфликты операции",
        "parent shas:" => "родительские SHA:",
        "passphrase" => "passphrase",
        "push:" => "push:",
        "rebase" => "rebase",
        "reflog" => "reflog",
        "refs" => "refs",
        "remote" => "удалённый",
        "remote:" => "удалённый:",
        "remove selected worktree?" => "удалить выбранный worktree?",
        "reset layout" => "сбросить макет",
        "resolve conflicts in your editor, then action+Shift+C" => "разрешите конфликты в редакторе, затем action+Shift+C",
        "resolve files externally, then action+Shift+C" => "разрешите файлы вне приложения, затем action+Shift+C",
        "revert" => "revert",
        "select a branch to checkout" => "выберите ветку для checkout",
        "select a branch to delete" => "выберите ветку для удаления",
        "select a branch to rename" => "выберите ветку для переименования",
        "select a branch to solo" => "выберите ветку для изоляции",
        "select a branch to toggle" => "выберите ветку для переключения",
        "select a tag to delete" => "выберите тег для удаления",
        "select a worktree to open" => "выберите worktree для открытия",
        "select a worktree to remove" => "выберите worktree для удаления",
        "select remote to manage | + add remote to create " => "выберите удалённый для управления | + добавить удалённый для создания ",
        "session only " => "только сеанс ",
        "set shortcut" => "задать сочетание",
        "ssh-agent when available " => "ssh-agent при наличии ",
        "stash" => "stash",
        "stashes" => "stash",
        "switch field" => "сменить поле",
        "tab" => "tab",
        "user:" => "пользователь:",
        "username/password or token prompt " => "запрос имени пользователя/пароля или токена ",
        "worktrees" => "worktree",
        _ => en,
    }
}

fn tr_extra(en: &'static str) -> &'static str {
    match en {
        " + add remote" => " + remote ekle",
        " active custom symbols:" => " aktif özel semboller:",
        " active custom:" => " aktif özel:",
        " authorization:" => " yetkilendirme:",
        " email:" => " e-posta:",
        " graph metadata:" => " grafik meta verisi:",
        " https:" => " https:",
        " keymap:" => " tuş eşlemesi:",
        " layout:" => " düzen:",
        " name:" => " ad:",
        " no matches" => " eşleşme yok",
        " paths:" => " yollar:",
        " recent file:" => " son dosya:",
        " remote error:" => " remote hatası:",
        " secrets:" => " gizli bilgiler:",
        " settings" => " ayarlar",
        " shortcuts / action mode:" => " kısayollar / eylem modu:",
        " shortcuts / normal mode:" => " kısayollar / normal mod:",
        " ssh fallback:" => " ssh yedeği:",
        " status" => " durum",
        " symbol theme:" => " sembol teması:",
        " symbols:" => " semboller:",
        " theme:" => " tema:",
        " type to search" => " aramak için yaz",
        "! not a valid git repository !" => "! geçerli bir Git deposu değil !",
        "(enter)" => "(enter)",
        "Abort failed: no rebase, cherry-pick, revert, or merge in progress" => "İptal başarısız: sürmekte olan rebase, cherry-pick, revert veya merge yok",
        "Action mode" => "Eylem modu",
        "Add remote failed: remote name is invalid" => "Remote ekleme başarısız: remote adı geçersiz",
        "Checkout" => "Checkout",
        "Checkout branch" => "Branch checkout",
        "Cherry-pick" => "Cherry-pick",
        "Cherry-pick aborted." => "Cherry-pick iptal edildi.",
        "Cherry-pick commit" => "Cherry-pick commit",
        "Cherry-pick completed." => "Cherry-pick tamamlandı.",
        "Cherry-pick failed: no commit is pending" => "Cherry-pick başarısız: bekleyen commit yok",
        "Cherry-pick failed: no commit message was provided" => "Cherry-pick başarısız: commit mesajı verilmedi",
        "Cherry-pick stopped because conflicts need to be resolved." => "Cherry-pick durdu: çakışmalar çözülmeli.",
        "Commit" => "Commit",
        "Continue failed: no rebase, cherry-pick, revert, or merge in progress" => "Devam başarısız: sürmekte olan rebase, cherry-pick, revert veya merge yok",
        "Continue operation" => "Operasyona devam et",
        "Create branch failed: no commit is selected" => "Dal oluşturma başarısız: commit seçilmedi",
        "Create branch here" => "Burada dal oluştur",
        "Create tag failed: no commit is selected" => "Etiket oluşturma başarısız: commit seçilmedi",
        "Create worktree" => "Worktree oluştur",
        "Create worktree failed: names cannot be empty or contain path separators" => "Worktree oluşturma başarısız: adlar boş olamaz veya yol ayırıcı içeremez",
        "Create worktree failed: no commit is selected" => "Worktree oluşturma başarısız: commit seçilmedi",
        "Create worktree failed: path cannot be empty" => "Worktree oluşturma başarısız: yol boş olamaz",
        "Delete branch failed: cannot delete the current branch" => "Dal silme başarısız: geçerli dal silinemez",
        "Delete branch failed: remote branch name is invalid" => "Dal silme başarısız: remote dal adı geçersiz",
        "Delete remote branch" => "Remote dalı sil",
        "Delete remote failed: no remote is pending" => "Remote silme başarısız: bekleyen remote yok",
        "Discard file changes" => "Dosya değişikliklerini at",
        "Drop stash" => "Stash sil",
        "Drop stash failed" => "Stash silme başarısız",
        "Edit fetch URL" => "Fetch URL düzenle",
        "Edit push URL" => "Push URL düzenle",
        "Edit remote failed" => "Remote düzenleme başarısız",
        "Edit remote failed: no remote is pending" => "Remote düzenleme başarısız: bekleyen remote yok",
        "Enter cherry-pick commit message" => "Cherry-pick commit mesajını gir",
        "Enter commit SHA to search for" => "Aranacak commit SHA değerini gir",
        "Enter graph lane limit" => "Grafik şerit sınırını gir",
        "Enter commit message" => "Commit mesajını gir",
        "Enter lock reason" => "Kilit nedenini gir",
        "Enter new branch name" => "Yeni dal adını gir",
        "Enter new remote URL" => "Yeni remote URL gir",
        "Enter new remote name" => "Yeni remote adını gir",
        "Enter new tag name" => "Yeni etiket adını gir",
        "Enter new worktree name" => "Yeni worktree adını gir",
        "Enter new worktree path" => "Yeni worktree yolunu gir",
        "Enter remote fetch URL" => "Remote fetch URL gir",
        "Enter remote push URL" => "Remote push URL gir",
        "Enter renamed branch name" => "Yeniden adlandırılmış dal adını gir",
        "Enter renamed remote name" => "Yeniden adlandırılmış remote adını gir",
        "Enter revert commit message" => "Revert commit mesajını gir",
        "Fetch" => "Fetch",
        "Fetch all" => "Tümünü fetch et",
        "File history failed: graph worker is unavailable" => "Dosya geçmişi başarısız: grafik worker kullanılamıyor",
        "Focus next pane" => "Sonraki panele odaklan",
        "Focus pane down" => "Aşağıdaki panele odaklan",
        "Focus pane left" => "Soldaki panele odaklan",
        "Focus pane right" => "Sağdaki panele odaklan",
        "Focus pane up" => "Üstteki panele odaklan",
        "Focus previous pane" => "Önceki panele odaklan",
        "Git network operation failed: another network operation is already running" => "Git ağ işlemi başarısız: başka bir ağ işlemi zaten çalışıyor",
        "Git network operation failed: worker thread panicked" => "Git ağ işlemi başarısız: worker thread panikledi",
        "Git operation failed: no repository is open" => "Git işlemi başarısız: açık depo yok",
        "Hard reset" => "Hard reset",
        "Hard reset failed" => "Hard reset başarısız",
        "Lock worktree" => "Worktree kilitle",
        "Lock worktree failed" => "Worktree kilitleme başarısız",
        "Lock worktree failed: only valid linked worktrees can be locked" => "Worktree kilitleme başarısız: yalnızca geçerli bağlı worktree kilitlenebilir",
        "Merge" => "Merge",
        "Merge aborted." => "Merge iptal edildi.",
        "Merge already up to date." => "Merge zaten güncel.",
        "Merge completed." => "Merge tamamlandı.",
        "Merge failed" => "Merge başarısız",
        "Merge fast-forwarded." => "Merge fast-forward yapıldı.",
        "Merge stopped because conflicts need to be resolved." => "Merge durdu: çakışmalar çözülmeli.",
        "Minimize" => "Küçült",
        "Mixed reset" => "Mixed reset",
        "Mixed reset failed" => "Mixed reset başarısız",
        "Move recent repository down" => "Son depoyu aşağı taşı",
        "Move recent repository up" => "Son depoyu yukarı taşı",
        "Narrow scope" => "Kapsamı daralt",
        "Open stash commit" => "Stash commit aç",
        "Open submodule" => "Alt modülü aç",
        "Open submodule failed: submodule is not initialized. Run update/init first." => "Alt modül açılamadı: alt modül başlatılmadı. Önce update/init çalıştırın.",
        "Open worktree" => "Worktree aç",
        "Open worktree failed: worktree path is invalid" => "Worktree açılamadı: worktree yolu geçersiz",
        "Pop stash" => "Stash uygula",
        "Pop stash failed" => "Stash uygulama başarısız",
        "Push" => "Push",
        "Push failed: detached HEAD has no current branch" => "Push başarısız: detached HEAD için geçerli dal yok",
        "Push tags" => "Etiketleri push et",
        "Rebase" => "Rebase",
        "Rebase aborted." => "Rebase iptal edildi.",
        "Rebase failed" => "Rebase başarısız",
        "Rebase stopped because conflicts need to be resolved." => "Rebase durdu: çakışmalar çözülmeli.",
        "Reflog commit is hidden from the graph. Press 9 to show graph reflogs." => "Reflog commit grafikte gizli. Grafik refloglarını göstermek için 9’a basın.",
        "Reload all branches" => "Tüm dalları yeniden yükle",
        "Remove recent repository" => "Son depoyu kaldır",
        "Remove worktree" => "Worktree kaldır",
        "Remove worktree failed" => "Worktree kaldırma başarısız",
        "Remove worktree failed: cannot remove current, main, or locked worktrees" => "Worktree kaldırma başarısız: geçerli, ana veya kilitli worktree kaldırılamaz",
        "Rename branch failed" => "Dal yeniden adlandırma başarısız",
        "Rename branch failed: no branch is pending" => "Dal yeniden adlandırma başarısız: bekleyen dal yok",
        "Rename branch failed: only local branches can be renamed" => "Dal yeniden adlandırma başarısız: yalnızca yerel dallar yeniden adlandırılabilir",
        "Rename remote failed" => "Remote yeniden adlandırma başarısız",
        "Rename remote failed: no remote is pending" => "Remote yeniden adlandırma başarısız: bekleyen remote yok",
        "Reset file failed" => "Dosya sıfırlama başarısız",
        "Reset layout" => "Düzeni sıfırla",
        "Resize pane down" => "Paneli aşağı yeniden boyutlandır",
        "Resize pane left" => "Paneli sola yeniden boyutlandır",
        "Resize pane right" => "Paneli sağa yeniden boyutlandır",
        "Resize pane up" => "Paneli yukarı yeniden boyutlandır",
        "Return to parent repository" => "Üst depoya dön",
        "Revert" => "Revert",
        "Revert aborted." => "Revert iptal edildi.",
        "Revert commit" => "Revert commit",
        "Revert completed." => "Revert tamamlandı.",
        "Revert failed" => "Revert başarısız",
        "Revert failed: no commit is pending" => "Revert başarısız: bekleyen commit yok",
        "Revert failed: no commit message was provided" => "Revert başarısız: commit mesajı verilmedi",
        "Revert failed: reverting merge commits is not supported" => "Revert başarısız: merge commitlerini revert etmek desteklenmiyor",
        "Revert stopped because conflicts need to be resolved." => "Revert durdu: çakışmalar çözülmeli.",
        "SHAs" => "SHA’lar",
        "Save keymap failed" => "Tuş eşlemesi kaydetme başarısız",
        "Scroll down branch" => "Sonraki dala kaydır",
        "Scroll down commit" => "Sonraki commit’e kaydır",
        "Scroll down half" => "Yarım sayfa aşağı kaydır",
        "Scroll half page down" => "Yarım sayfa aşağı kaydır",
        "Scroll half page up" => "Yarım sayfa yukarı kaydır",
        "Scroll page down" => "Sayfa aşağı kaydır",
        "Scroll page up" => "Sayfa yukarı kaydır",
        "Scroll up branch" => "Önceki dala kaydır",
        "Scroll up commit" => "Önceki commit’e kaydır",
        "Scroll up half" => "Yarım sayfa yukarı kaydır",
        "Search repository files" => "Depo dosyalarında ara",
        "Set as default" => "Varsayılan yap",
        "Set default remote failed" => "Varsayılan remote ayarlama başarısız",
        "Show files/status" => "Dosyaları/durumu göster",
        "Show full diff" => "Tam diff göster",
        "Show hunk rows" => "Hunk satırlarını göster",
        "Show split diff" => "Bölünmüş diff göster",
        "Show unified diff" => "Unified diff göster",
        "Solo branch" => "Dalı solo yap",
        "Splash screen" => "Başlangıç ekranı",
        "Stage all failed" => "Tümünü stage etme başarısız",
        "Stage file failed" => "Dosyayı stage etme başarısız",
        "Stage file failed: resolve conflicts in your editor, then continue the active operation" => "Dosyayı stage etme başarısız: çakışmaları editörde çözün, sonra aktif operasyona devam edin",
        "Stage submodule" => "Alt modülü stage et",
        "Stage submodule failed" => "Alt modülü stage etme başarısız",
        "Stash changes" => "Değişiklikleri stash’e al",
        "Stash failed" => "Stash başarısız",
        "Sync URL" => "URL senkronize et",
        "Sync submodule failed" => "Alt modül senkronizasyonu başarısız",
        "Shrink graph lane limit" => "Grafik şerit sınırını azalt",
        "Grow graph lane limit" => "Grafik şerit sınırını artır",
        "Toggle SHAs" => "SHA’ları aç/kapat",
        "Toggle branch" => "Dalı aç/kapat",
        "Toggle branches" => "Dalları aç/kapat",
        "Toggle graph committers" => "Grafik commit yapanlarını aç/kapat",
        "Toggle graph dates" => "Grafik tarihlerini aç/kapat",
        "Toggle graph reflogs" => "Grafik refloglarını aç/kapat",
        "Toggle graph refs" => "Grafik reflerini aç/kapat",
        "Toggle help" => "Yardımı aç/kapat",
        "Toggle hunk mode" => "Hunk modunu aç/kapat",
        "Toggle inspector" => "İnceleyiciyi aç/kapat",
        "Toggle reflogs" => "Reflogları aç/kapat",
        "Toggle search" => "Aramayı aç/kapat",
        "Toggle split diff mode" => "Bölünmüş diff modunu aç/kapat",
        "Toggle stashes" => "Stashleri aç/kapat",
        "Toggle status" => "Durumu aç/kapat",
        "Toggle submodules" => "Alt modülleri aç/kapat",
        "Toggle tags" => "Etiketleri aç/kapat",
        "Toggle worktree lock" => "Worktree kilidini aç/kapat",
        "Toggle worktrees" => "Worktree’leri aç/kapat",
        "Toggle zen mode" => "Zen modunu aç/kapat",
        "Unlock worktree" => "Worktree kilidini aç",
        "Unlock worktree failed" => "Worktree kilidi açma başarısız",
        "Unstage all failed" => "Tümünü stage’den çıkarma başarısız",
        "Unstage file failed" => "Dosyayı stage’den çıkarma başarısız",
        "Unstage file failed: resolve conflicts in your editor, then continue the active operation" => {
            "Dosyayı stage’den çıkarma başarısız: çakışmaları editörde çözün, sonra aktif operasyona devam edin"
        },
        "Unstage submodule" => "Alt modülü stage’den çıkar",
        "Unstage submodule failed" => "Alt modülü stage’den çıkarma başarısız",
        "Unsupported" => "Desteklenmiyor",
        "Update submodule" => "Alt modülü güncelle",
        "Update/init submodule" => "Alt modülü güncelle/başlat",
        "Widen scope" => "Kapsamı genişlet",
        "actions:" => "eylemler:",
        "auth" => "auth",
        "cherrypick" => "cherry-pick",
        "committer date/time" => "commit yapan tarih/saat",
        "committers" => "commit yapanlar",
        "current:" => "geçerli:",
        "delete selected remote?" => "seçili remote silinsin mi?",
        "detached head:" => "detached HEAD:",
        "enter" => "enter",
        "featured branches:" => "öne çıkan dallar:",
        "fetch:" => "fetch:",
        "graph reflog commits" => "grafik reflog commitleri",
        "head reflog:" => "HEAD reflog:",
        "inspector" => "inceleyici",
        "key passphrase prompt " => "anahtar passphrase istemi ",
        "key:" => "anahtar:",
        "local" => "yerel",
        "merge" => "merge",
        "modal" => "modal",
        "move" => "taşı",
        "move down" => "aşağı taşı",
        "move up" => "yukarı taşı",
        "new:" => "yeni:",
        "new: waiting for key" => "yeni: tuş bekleniyor",
        "no head (no commits yet)" => "HEAD yok (henüz commit yok)",
        "ok" => "tamam",
        "operation conflicts" => "operasyon çakışmaları",
        "parent shas:" => "üst SHA’lar:",
        "passphrase" => "passphrase",
        "push:" => "push:",
        "rebase" => "rebase",
        "reflog" => "reflog",
        "refs" => "refler",
        "remote" => "remote",
        "remote:" => "remote:",
        "remove selected worktree?" => "seçili worktree kaldırılsın mı?",
        "reset layout" => "düzeni sıfırla",
        "resolve conflicts in your editor, then action+Shift+C" => "çakışmaları editörde çözün, sonra action+Shift+C",
        "resolve files externally, then action+Shift+C" => "dosyaları dışarıda çözün, sonra action+Shift+C",
        "revert" => "revert",
        "select a branch to checkout" => "checkout için dal seç",
        "select a branch to delete" => "silmek için dal seç",
        "select a branch to rename" => "yeniden adlandırmak için dal seç",
        "select a branch to solo" => "solo yapmak için dal seç",
        "select a branch to toggle" => "aç/kapat için dal seç",
        "select a tag to delete" => "silmek için etiket seç",
        "select a worktree to open" => "açmak için worktree seç",
        "select a worktree to remove" => "kaldırmak için worktree seç",
        "select remote to manage | + add remote to create " => "yönetilecek remote seç | oluşturmak için + remote ekle ",
        "session only " => "yalnızca oturum ",
        "set shortcut" => "kısayol ayarla",
        "ssh-agent when available " => "mevcutsa ssh-agent ",
        "stash" => "stash",
        "stashes" => "stashler",
        "switch field" => "alan değiştir",
        "tab" => "tab",
        "user:" => "kullanıcı:",
        "username/password or token prompt " => "kullanıcı adı/parola veya token istemi ",
        "worktrees" => "worktree’ler",
        _ => en,
    }
}

macro_rules! localized_module {
    ($module:ident { $($name:ident => $text:literal),+ $(,)? }) => {
        pub mod $module {
            use super::tr;
            $(pub fn $name() -> &'static str { tr($text) })+
        }
    };
}

macro_rules! localized_fns {
    ($($name:ident => $text:literal),+ $(,)?) => {
        $(pub fn $name() -> &'static str { tr($text) })+
    };
}

localized_module!(common {
    DEFAULT_REMOTE => "default",
    LOADING => "loading",
    NONE => "none",
    NO_HEAD => "no head",
    NOT_INITIALIZED => "not initialized",
    UNKNOWN => "-",
    WORKING => "working...",
});

localized_module!(empty {
    NO_BODY => "no body",
    NO_BRANCHES => "no branches",
    NO_COMMITS => "no commits",
    NO_HEAD_REFLOG => "no HEAD reflog",
    NO_MESSAGE => "no message",
    NO_RECENT_REPOSITORIES => "no recent repositories",
    NO_REMOTES => "no remotes",
    NO_STAGED_CHANGES => "no staged changes",
    NO_STASHES => "no stashes",
    NO_SUBMODULES => "no submodules",
    NO_SUMMARY => "no summary",
    NO_TAGS => "no tags",
    NO_UNSTAGED_CHANGES => "no unstaged changes",
    NO_WORKTREES => "no worktrees",
    SEARCH => "search",
});

pub mod errors {
    use super::{Display, Language, active_language, tr};

    pub fn ABORT_NO_OPERATION() -> &'static str {
        tr("Abort failed: no rebase, cherry-pick, revert, or merge in progress")
    }
    pub fn ADD_REMOTE_INVALID_NAME() -> &'static str {
        tr("Add remote failed: remote name is invalid")
    }
    pub fn ADD_REMOTE() -> &'static str {
        tr("Add remote failed")
    }
    pub fn CHECKOUT() -> &'static str {
        tr("Checkout failed")
    }
    pub fn CHERRYPICK() -> &'static str {
        tr("Cherry-pick failed")
    }
    pub fn CHERRYPICK_NO_MESSAGE() -> &'static str {
        tr("Cherry-pick failed: no commit message was provided")
    }
    pub fn CHERRYPICK_NO_PENDING() -> &'static str {
        tr("Cherry-pick failed: no commit is pending")
    }
    pub fn COMMIT() -> &'static str {
        tr("Commit failed")
    }
    pub fn CONTINUE_NO_OPERATION() -> &'static str {
        tr("Continue failed: no rebase, cherry-pick, revert, or merge in progress")
    }
    pub fn CREATE_BRANCH() -> &'static str {
        tr("Create branch failed")
    }
    pub fn CREATE_BRANCH_NO_COMMIT() -> &'static str {
        tr("Create branch failed: no commit is selected")
    }
    pub fn CREATE_TAG() -> &'static str {
        tr("Create tag failed")
    }
    pub fn CREATE_TAG_NO_COMMIT() -> &'static str {
        tr("Create tag failed: no commit is selected")
    }
    pub fn CREATE_WORKTREE() -> &'static str {
        tr("Create worktree failed")
    }
    pub fn CREATE_WORKTREE_INVALID_NAME() -> &'static str {
        tr("Create worktree failed: names cannot be empty or contain path separators")
    }
    pub fn CREATE_WORKTREE_EMPTY_PATH() -> &'static str {
        tr("Create worktree failed: path cannot be empty")
    }
    pub fn CREATE_WORKTREE_NO_COMMIT() -> &'static str {
        tr("Create worktree failed: no commit is selected")
    }
    pub fn DELETE_BRANCH() -> &'static str {
        tr("Delete branch failed")
    }
    pub fn DELETE_BRANCH_CURRENT() -> &'static str {
        tr("Delete branch failed: cannot delete the current branch")
    }
    pub fn DELETE_BRANCH_INVALID_REMOTE() -> &'static str {
        tr("Delete branch failed: remote branch name is invalid")
    }
    pub fn DELETE_REMOTE() -> &'static str {
        tr("Delete remote failed")
    }
    pub fn DELETE_REMOTE_NO_PENDING() -> &'static str {
        tr("Delete remote failed: no remote is pending")
    }
    pub fn DELETE_TAG() -> &'static str {
        tr("Delete tag failed")
    }
    pub fn DROP_STASH() -> &'static str {
        tr("Drop stash failed")
    }
    pub fn EDIT_REMOTE() -> &'static str {
        tr("Edit remote failed")
    }
    pub fn EDIT_REMOTE_NO_PENDING() -> &'static str {
        tr("Edit remote failed: no remote is pending")
    }
    pub fn FILE_DIFF() -> &'static str {
        tr("Couldn't get the file diff")
    }
    pub fn FILE_HISTORY_WORKER_UNAVAILABLE() -> &'static str {
        tr("File history failed: graph worker is unavailable")
    }
    pub fn GIT_NETWORK_ALREADY_RUNNING() -> &'static str {
        tr("Git network operation failed: another network operation is already running")
    }
    pub fn GIT_NETWORK_PANICKED() -> &'static str {
        tr("Git network operation failed: worker thread panicked")
    }
    pub fn GIT_OPERATION_NO_REPOSITORY() -> &'static str {
        tr("Git operation failed: no repository is open")
    }
    pub fn HARD_RESET() -> &'static str {
        tr("Hard reset failed")
    }
    pub fn LOCK_WORKTREE() -> &'static str {
        tr("Lock worktree failed")
    }
    pub fn LOCK_WORKTREE_INVALID() -> &'static str {
        tr("Lock worktree failed: only valid linked worktrees can be locked")
    }
    pub fn MERGE() -> &'static str {
        tr("Merge failed")
    }
    pub fn MIXED_RESET() -> &'static str {
        tr("Mixed reset failed")
    }
    pub fn OPEN_REPOSITORY() -> &'static str {
        tr("Open repository failed")
    }
    pub fn OPEN_SUBMODULE_NOT_INITIALIZED() -> &'static str {
        tr("Open submodule failed: submodule is not initialized. Run update/init first.")
    }
    pub fn OPEN_WORKTREE_INVALID_PATH() -> &'static str {
        tr("Open worktree failed: worktree path is invalid")
    }
    pub fn POP_STASH() -> &'static str {
        tr("Pop stash failed")
    }
    pub fn PUSH_DETACHED_HEAD() -> &'static str {
        tr("Push failed: detached HEAD has no current branch")
    }
    pub fn REBASE() -> &'static str {
        tr("Rebase failed")
    }
    pub fn REMOVE_WORKTREE() -> &'static str {
        tr("Remove worktree failed")
    }
    pub fn REMOVE_WORKTREE_FORBIDDEN() -> &'static str {
        tr("Remove worktree failed: cannot remove current, main, or locked worktrees")
    }
    pub fn RENAME_BRANCH() -> &'static str {
        tr("Rename branch failed")
    }
    pub fn RENAME_BRANCH_LOCAL_ONLY() -> &'static str {
        tr("Rename branch failed: only local branches can be renamed")
    }
    pub fn RENAME_BRANCH_NO_PENDING() -> &'static str {
        tr("Rename branch failed: no branch is pending")
    }
    pub fn RENAME_REMOTE() -> &'static str {
        tr("Rename remote failed")
    }
    pub fn RENAME_REMOTE_NO_PENDING() -> &'static str {
        tr("Rename remote failed: no remote is pending")
    }
    pub fn REFLOG_COMMIT_HIDDEN() -> &'static str {
        tr("Reflog commit is hidden from the graph. Press 9 to show graph reflogs.")
    }
    pub fn RESET_FILE() -> &'static str {
        tr("Reset file failed")
    }
    pub fn REVERT() -> &'static str {
        tr("Revert failed")
    }
    pub fn REVERT_MERGE_UNSUPPORTED() -> &'static str {
        tr("Revert failed: reverting merge commits is not supported")
    }
    pub fn REVERT_NO_MESSAGE() -> &'static str {
        tr("Revert failed: no commit message was provided")
    }
    pub fn REVERT_NO_PENDING() -> &'static str {
        tr("Revert failed: no commit is pending")
    }
    pub fn SAVE_KEYMAP() -> &'static str {
        tr("Save keymap failed")
    }
    pub fn SET_DEFAULT_REMOTE() -> &'static str {
        tr("Set default remote failed")
    }
    pub fn STAGE_ALL() -> &'static str {
        tr("Stage all failed")
    }
    pub fn STAGE_FILE() -> &'static str {
        tr("Stage file failed")
    }
    pub fn STAGE_FILE_CONFLICT() -> &'static str {
        tr("Stage file failed: resolve conflicts in your editor, then continue the active operation")
    }
    pub fn STAGE_SUBMODULE() -> &'static str {
        tr("Stage submodule failed")
    }
    pub fn STASH() -> &'static str {
        tr("Stash failed")
    }
    pub fn SYNC_SUBMODULE() -> &'static str {
        tr("Sync submodule failed")
    }
    pub fn UNSTAGE_ALL() -> &'static str {
        tr("Unstage all failed")
    }
    pub fn UNSTAGE_FILE() -> &'static str {
        tr("Unstage file failed")
    }
    pub fn UNSTAGE_FILE_CONFLICT() -> &'static str {
        tr("Unstage file failed: resolve conflicts in your editor, then continue the active operation")
    }
    pub fn UNSTAGE_SUBMODULE() -> &'static str {
        tr("Unstage submodule failed")
    }
    pub fn UNLOCK_WORKTREE() -> &'static str {
        tr("Unlock worktree failed")
    }

    pub fn with_error(prefix: &str, error: impl Display) -> String {
        format!("{prefix}: {error}")
    }

    pub fn authentication_failed(operation: &str, attempts: usize) -> String {
        match active_language() {
            Language::Spanish => format!("{operation} falló: autenticación fallida tras {attempts} intentos"),
            Language::French => format!("{operation} a échoué : authentification échouée après {attempts} tentatives"),
            Language::Russian => format!("{operation} не удалось: ошибка аутентификации после {attempts} попыток"),
            Language::Turkish => format!("{operation} başarısız: {attempts} denemeden sonra kimlik doğrulama başarısız"),
            Language::English => format!("{operation} failed: authentication failed after {attempts} attempts"),
        }
    }

    pub fn auth_cancelled(operation: &str) -> String {
        match active_language() {
            Language::Spanish => format!("{operation} cancelado: no se proporcionó autenticación"),
            Language::French => format!("{operation} annulé : authentification non fournie"),
            Language::Russian => format!("{operation} отменено: аутентификация не предоставлена"),
            Language::Turkish => format!("{operation} iptal edildi: kimlik doğrulama sağlanmadı"),
            Language::English => format!("{operation} cancelled: authentication was not provided"),
        }
    }

    pub fn no_remotes_configured(operation: &str) -> String {
        match active_language() {
            Language::Spanish => format!("{operation} falló: no hay remotos configurados"),
            Language::French => format!("{operation} a échoué : aucun distant configuré"),
            Language::Russian => format!("{operation} не удалось: удалённые не настроены"),
            Language::Turkish => format!("{operation} başarısız: remote yapılandırılmamış"),
            Language::English => format!("{operation} failed: no remotes configured"),
        }
    }

    pub fn operation_failed(operation: &str, error: impl Display) -> String {
        match active_language() {
            Language::Spanish => format!("{operation} falló: {error}"),
            Language::French => format!("{operation} a échoué : {error}"),
            Language::Russian => format!("{operation} не удалось: {error}"),
            Language::Turkish => format!("{operation} başarısız: {error}"),
            Language::English => format!("{operation} failed: {error}"),
        }
    }

    pub fn walker_failed(error: impl Display) -> String {
        match active_language() {
            Language::Spanish => format!("Walker falló: {error}"),
            Language::French => format!("Walker échoué : {error}"),
            Language::Russian => format!("Walker не удался: {error}"),
            Language::Turkish => format!("Walker başarısız: {error}"),
            Language::English => format!("Walker failed: {error}"),
        }
    }
}

localized_module!(inspector {
    AUTHORED_BY => "authored by:",
    COMMIT_SHA => "commit sha:",
    COMMITTED_BY => "committed by:",
    CONFLICTED_FILES => "conflicted files:",
    FEATURED_BRANCHES => "featured branches:",
    HEAD_REFLOG => "head reflog:",
    MESSAGE_BODY => "message body:",
    MESSAGE_SUMMARY => "message summary:",
    NEXT_ACTION => "next action:",
    OPERATION_CONFLICTS => "operation conflicts",
    PARENT_SHAS => "parent shas:",
    REPOSITORY_STATE => "repository state:",
    RESOLVE_CONFLICTS_ACTION => "resolve files externally, then action+Shift+C",
});

localized_module!(keymap {
    ACTION_MODE => "action",
    ALT => "Alt",
    BACK_TAB => "BackTab",
    BACKSPACE => "Backspace",
    CAPS_LOCK => "CapsLock",
    CHAR => "Char",
    COMMAND => "Command",
    CONTROL => "Control",
    CTRL => "Ctrl",
    DELETE => "Delete",
    DOWN => "Down",
    END => "End",
    ENTER => "Enter",
    ESC => "Esc",
    HOME => "Home",
    INSERT => "Insert",
    LEFT => "Left",
    META => "Meta",
    NORMAL_MODE => "normal",
    NULL => "Null",
    NUM_LOCK => "NumLock",
    PAGE_DOWN => "PageDown",
    PAGE_UP => "PageUp",
    PAUSE => "Pause",
    PRINT_SCREEN => "PrintScreen",
    RIGHT => "Right",
    SCROLL_LOCK => "ScrollLock",
    SHIFT => "Shift",
    SPACE => "Space",
    TAB => "Tab",
    UNSUPPORTED => "Unsupported",
    UP => "Up",
});

pub mod menu {
    use super::{Language, active_language, tr};

    localized_fns! {
    ABORT_OPERATION => "Abort operation",
    ADD_REMOTE => "Add remote",
    APPLY_THEME => "Apply theme",
    APPLY_LANGUAGE => "Apply language",
    BACK => "Back",
    BACK_TO_GRAPH => "Back to graph",
    CHECKOUT => "Checkout",
    CHECKOUT_BRANCH => "Checkout branch",
    CHERRYPICK => "Cherry-pick",
    COMMIT => "Commit",
    CONTINUE_OPERATION => "Continue operation",
    CREATE_BRANCH => "Create branch",
    CREATE_BRANCH_HERE => "Create branch here",
    CREATE_TAG => "Create tag",
    CREATE_WORKTREE => "Create worktree",
    DELETE_BRANCH => "Delete branch",
    DELETE_REMOTE => "Delete remote",
    DELETE_TAG => "Delete tag",
    DISCARD_FILE_CHANGES => "Discard file changes",
    DROP_STASH => "Drop stash",
    EDIT_FETCH_URL => "Edit fetch URL",
    EDIT_PUSH_URL => "Edit push URL",
    EXIT => "Exit",
    FETCH => "Fetch",
    FIND => "Find",
    FIND_FILE => "Find file",
    HARD_RESET => "Hard reset",
    LOCK_WORKTREE => "Lock worktree",
    MERGE => "Merge",
    MIXED_RESET => "Mixed reset",
    MOVE_DOWN => "Move down",
    MOVE_UP => "Move up",
    OPEN_COMMIT => "Open commit",
    OPEN_FILE => "Open file",
    OPEN_REPOSITORY => "Open repository",
    OPEN_STASH_COMMIT => "Open stash commit",
    OPEN_SUBMODULE => "Open submodule",
    OPEN_WORKTREE => "Open worktree",
    POP_STASH => "Pop stash",
    PUSH => "Push",
    REBASE => "Rebase",
    REBIND_SHORTCUT => "Rebind shortcut",
    RELOAD => "Reload",
    REMOVE => "Remove",
    REMOVE_WORKTREE => "Remove worktree",
    RENAME_BRANCH => "Rename branch",
    RENAME_REMOTE => "Rename remote",
    RETURN_TO_PARENT_REPOSITORY => "Return to parent repository",
    REVERT => "Revert",
    SET_AS_DEFAULT => "Set as default",
    SETTINGS => "Settings",
    SHOW_DETAILS => "Show details",
    SHOW_FILES_STATUS => "Show files/status",
    SHOW_FULL_DIFF => "Show full diff",
    SHOW_HUNK_ROWS => "Show hunk rows",
    SHOW_SPLIT_DIFF => "Show split diff",
    SHOW_UNIFIED_DIFF => "Show unified diff",
    SOLO_BRANCH => "Solo branch",
    SPLASH_SCREEN => "Splash screen",
    STAGE_ALL => "Stage all",
    STAGE_FILE => "Stage file",
    STAGE_SUBMODULE => "Stage submodule",
    STASH_CHANGES => "Stash changes",
    SYNC_URL => "Sync URL",
    TOGGLE_BRANCH => "Toggle branch",
    UNLOCK_WORKTREE => "Unlock worktree",
    UNSTAGE_ALL => "Unstage all",
    UNSTAGE_FILE => "Unstage file",
    UNSTAGE_SUBMODULE => "Unstage submodule",
    UPDATE_INIT_SUBMODULE => "Update/init submodule",
    }

    pub fn open_settings_tab(tab: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Abrir {tab}"),
            Language::French => format!("Ouvrir {tab}"),
            Language::Russian => format!("Открыть {tab}"),
            Language::Turkish => format!("{tab} aç"),
            Language::English => format!("Open {tab}"),
        }
    }

    pub fn run_command(command: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Ejecutar {command}"),
            Language::French => format!("Exécuter {command}"),
            Language::Russian => format!("Выполнить {command}"),
            Language::Turkish => format!("{command} çalıştır"),
            Language::English => format!("Run {command}"),
        }
    }
}

pub mod modal {
    use super::{Language, active_language, tr};

    localized_fns! {
    ACTION_CHOOSE => "choose",
    ACTION_CONFIRM => "confirm",
    ACTION_MOVE => "move",
    ACTION_OK => "ok",
    ACTION_SAVE => "save",
    ACTION_SUBMIT => "submit",
    ACTION_SWITCH_FIELD => "switch field",
    AUTH_KEY => "key:",
    AUTH_PASSPHRASE => "passphrase",
    AUTH_PASSWORD_TOKEN => "password / token",
    AUTH_USER => "user:",
    AUTH_USERNAME => "username",
    CURRENT_SHORTCUT => "current:",
    DELETE_SELECTED_REMOTE => "delete selected remote?",
    ERROR_TITLE => "error",
    KEY_ENTER => "enter",
    KEY_TAB => "tab",
    KEY_CTRL_J_K => "ctrl+j/k",
    NAME_LABEL => "name:",
    NEW_SHORTCUT => "new:",
    NEW_SHORTCUT_WAITING => "new: waiting for key",
    PATH_LABEL => "path:",
    PRESS_KEY => "press key",
    PROMPT_CHERRYPICK_COMMIT => "Enter cherry-pick commit message",
    PROMPT_CREATE_BRANCH => "Enter new branch name",
    PROMPT_CREATE_COMMIT => "Enter commit message",
    PROMPT_CREATE_TAG => "Enter new tag name",
    PROMPT_CREATE_WORKTREE_NAME => "Enter new worktree name",
    PROMPT_CREATE_WORKTREE_PATH => "Enter new worktree path",
    PROMPT_FIND_FILE => "Search repository files",
    PROMPT_FIND_SHA => "Enter commit SHA to search for",
    PROMPT_GRAPH_LANE_LIMIT => "Enter graph lane limit",
    PROMPT_LOCK_WORKTREE => "Enter lock reason",
    PROMPT_REMOTE_ADD_NAME => "Enter new remote name",
    PROMPT_REMOTE_ADD_URL => "Enter new remote URL",
    PROMPT_REMOTE_EDIT_PUSH_URL => "Enter remote push URL",
    PROMPT_REMOTE_EDIT_URL => "Enter remote fetch URL",
    PROMPT_REMOTE_RENAME => "Enter renamed remote name",
    PROMPT_RENAME_BRANCH => "Enter renamed branch name",
    PROMPT_REVERT_COMMIT => "Enter revert commit message",
    REMOTE_FALLBACK => "remote",
    REMOTE_LABEL => "remote:",
    REMOVE_SELECTED_WORKTREE => "remove selected worktree?",
    SELECT_BRANCH_CHECKOUT => "select a branch to checkout",
    SELECT_BRANCH_DELETE => "select a branch to delete",
    SELECT_BRANCH_RENAME => "select a branch to rename",
    SELECT_BRANCH_SOLO => "select a branch to solo",
    SELECT_BRANCH_TOGGLE => "select a branch to toggle",
    SELECT_TAG_DELETE => "select a tag to delete",
    SELECT_WORKTREE_OPEN => "select a worktree to open",
    SELECT_WORKTREE_REMOVE => "select a worktree to remove",
    SET_SHORTCUT => "set shortcut",
    TYPE_TO_SEARCH => " type to search",
    NO_MATCHES => " no matches",
    }

    pub fn auth_title(protocol: &str) -> String {
        match active_language() {
            Language::Spanish => format!("autenticación {protocol}"),
            Language::French => format!("authentification {protocol}"),
            Language::Russian => format!("аутентификация {protocol}"),
            Language::Turkish => format!("{protocol} kimlik doğrulaması"),
            Language::English => format!("{protocol} authentication"),
        }
    }

    pub fn keymap_conflict(mode: &str, key: &str, command: &str) -> String {
        match active_language() {
            Language::Spanish => format!("conflicto: {mode} {key} ya ejecuta {command}"),
            Language::French => format!("conflit : {mode} {key} exécute déjà {command}"),
            Language::Russian => format!("конфликт: {mode} {key} уже выполняет {command}"),
            Language::Turkish => format!("çakışma: {mode} {key} zaten {command} çalıştırıyor"),
            Language::English => format!("conflict: {mode} {key} already runs {command}"),
        }
    }

    pub fn keymap_missing_mode(mode: &str) -> String {
        match active_language() {
            Language::Spanish => format!("modo de mapa de teclas faltante: {mode}"),
            Language::French => format!("mode de clavier manquant : {mode}"),
            Language::Russian => format!("отсутствует режим раскладки: {mode}"),
            Language::Turkish => format!("eksik keymap modu: {mode}"),
            Language::English => format!("missing keymap mode: {mode}"),
        }
    }

    pub fn keymap_missing_binding(mode: &str, key: &str) -> String {
        match active_language() {
            Language::Spanish => format!("atajo faltante: {mode} {key}"),
            Language::French => format!("raccourci manquant : {mode} {key}"),
            Language::Russian => format!("отсутствует привязка: {mode} {key}"),
            Language::Turkish => format!("eksik bağlama: {mode} {key}"),
            Language::English => format!("missing binding: {mode} {key}"),
        }
    }

    pub fn keymap_binding_changed(mode: &str, key: &str, expected: &str, actual: &str) -> String {
        match active_language() {
            Language::Spanish => format!("atajo cambiado: {mode} {key} era {expected}, ahora {actual}"),
            Language::French => format!("raccourci modifié : {mode} {key} était {expected}, maintenant {actual}"),
            Language::Russian => format!("привязка изменена: {mode} {key} было {expected}, теперь {actual}"),
            Language::Turkish => format!("bağlama değişti: {mode} {key} {expected} idi, şimdi {actual}"),
            Language::English => format!("binding changed: {mode} {key} was {expected}, now {actual}"),
        }
    }
}

pub mod network {
    use super::{Language, active_language, tr};

    localized_fns! {
    DELETE_REMOTE_BRANCH => "Delete remote branch",
    FETCH => "Fetch",
    GIT_NETWORK_OPERATION => "Git network operation",
    PROTOCOL_HTTP => "HTTP",
    PROTOCOL_HTTPS => "HTTPS",
    PROTOCOL_LOCAL => "local",
    PROTOCOL_REMOTE => "remote",
    PROTOCOL_SSH => "SSH",
    PUSH => "Push",
    PUSH_TAGS => "Push tags",
    UPDATE_SUBMODULE => "Update submodule",
    }

    pub fn deleting_remote_branch(remote_name: &str, branch: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Eliminando {remote_name}/{branch}..."),
            Language::French => format!("Suppression de {remote_name}/{branch}..."),
            Language::Russian => format!("Удаление {remote_name}/{branch}..."),
            Language::Turkish => format!("{remote_name}/{branch} siliniyor..."),
            Language::English => format!("Deleting {remote_name}/{branch}..."),
        }
    }

    pub fn fetching(remote_name: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Fetch de {remote_name}..."),
            Language::French => format!("Fetch de {remote_name}..."),
            Language::Russian => format!("Fetch из {remote_name}..."),
            Language::Turkish => format!("{remote_name} fetch ediliyor..."),
            Language::English => format!("Fetching {remote_name}..."),
        }
    }

    pub fn force_pushing(branch: &str, remote_name: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Force push de {branch} a {remote_name}..."),
            Language::French => format!("Force push de {branch} vers {remote_name}..."),
            Language::Russian => format!("Force push {branch} в {remote_name}..."),
            Language::Turkish => format!("{branch}, {remote_name} hedefine force push ediliyor..."),
            Language::English => format!("Force pushing {branch} to {remote_name}..."),
        }
    }

    pub fn pushing(branch: &str, remote_name: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Push de {branch} a {remote_name}..."),
            Language::French => format!("Push de {branch} vers {remote_name}..."),
            Language::Russian => format!("Push {branch} в {remote_name}..."),
            Language::Turkish => format!("{branch}, {remote_name} hedefine push ediliyor..."),
            Language::English => format!("Pushing {branch} to {remote_name}..."),
        }
    }

    pub fn pushing_tags(remote_name: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Push de etiquetas locales a {remote_name}..."),
            Language::French => format!("Push des tags locaux vers {remote_name}..."),
            Language::Russian => format!("Push локальных тегов в {remote_name}..."),
            Language::Turkish => format!("Yerel etiketler {remote_name} hedefine push ediliyor..."),
            Language::English => format!("Pushing local tags to {remote_name}..."),
        }
    }

    pub fn updating_submodule(name: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Actualizando submódulo {name}..."),
            Language::French => format!("Mise à jour du sous-module {name}..."),
            Language::Russian => format!("Обновление подмодуля {name}..."),
            Language::Turkish => format!("{name} alt modülü güncelleniyor..."),
            Language::English => format!("Updating submodule {name}..."),
        }
    }
}

pub mod operations {
    use super::{Language, active_language, tr};

    localized_fns! {
    ABORTED => "aborted",
    CHERRYPICK => "cherrypick",
    CHERRYPICK_ABORTED => "Cherry-pick aborted.",
    CHERRYPICK_COMMIT_FALLBACK => "Cherry-pick commit",
    CHERRYPICK_COMPLETED => "Cherry-pick completed.",
    CHERRYPICK_CONFLICT => "Cherry-pick stopped because conflicts need to be resolved.",
    COMPLETE => "complete",
    CONFLICT => "conflict",
    MERGE => "merge",
    MERGE_ALREADY_UP_TO_DATE => "Merge already up to date.",
    MERGE_ABORTED => "Merge aborted.",
    MERGE_COMPLETED => "Merge completed.",
    MERGE_CONFLICT => "Merge stopped because conflicts need to be resolved.",
    MERGE_FAST_FORWARDED => "Merge fast-forwarded.",
    REBASE => "rebase",
    REBASE_ABORTED => "Rebase aborted.",
    REBASE_CONFLICT => "Rebase stopped because conflicts need to be resolved.",
    REVERT => "revert",
    REVERT_ABORTED => "Revert aborted.",
    REVERT_COMMIT_FALLBACK => "Revert commit",
    REVERT_COMPLETED => "Revert completed.",
    REVERT_CONFLICT => "Revert stopped because conflicts need to be resolved.",
    RESOLVE_CONFLICTS => "resolve conflicts in your editor, then action+Shift+C",
    }

    pub fn aborted(operation: &str) -> String {
        format!("{operation} {}.", ABORTED())
    }

    pub fn aborting(operation: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Abortando {operation}..."),
            Language::French => format!("Abandon de {operation}..."),
            Language::Russian => format!("Прерывание {operation}..."),
            Language::Turkish => format!("{operation} iptal ediliyor..."),
            Language::English => format!("Aborting {operation}..."),
        }
    }

    pub fn continuing(operation: &str) -> String {
        match active_language() {
            Language::Spanish => format!("Continuando {operation}..."),
            Language::French => format!("Continuation de {operation}..."),
            Language::Russian => format!("Продолжение {operation}..."),
            Language::Turkish => format!("{operation} devam ediyor..."),
            Language::English => format!("Continuing {operation}..."),
        }
    }

    pub fn cherrypicked(original_message: &str) -> String {
        match active_language() {
            Language::Spanish => format!("cherry-pick: {original_message}"),
            Language::French => format!("cherry-pick : {original_message}"),
            Language::Russian => format!("cherry-pick: {original_message}"),
            Language::Turkish => format!("cherry-pick: {original_message}"),
            Language::English => format!("cherrypicked: {original_message}"),
        }
    }

    pub fn rebase_completed(applied: usize) -> String {
        match active_language() {
            Language::Spanish if applied == 1 => "Rebase completado tras aplicar 1 commit.".to_string(),
            Language::Spanish => format!("Rebase completado tras aplicar {applied} commits."),
            Language::French if applied == 1 => "Rebase terminé après application d'un commit.".to_string(),
            Language::French => format!("Rebase terminé après application de {applied} commits."),
            Language::Russian if applied == 1 => "Rebase завершён после применения 1 коммита.".to_string(),
            Language::Russian => format!("Rebase завершён после применения {applied} коммитов."),
            Language::Turkish if applied == 1 => "Rebase 1 commit uygulandıktan sonra tamamlandı.".to_string(),
            Language::Turkish => format!("Rebase {applied} commit uygulandıktan sonra tamamlandı."),
            Language::English if applied == 1 => "Rebase completed after applying 1 commit.".to_string(),
            Language::English => format!("Rebase completed after applying {applied} commits."),
        }
    }

    pub fn reverted(original_message: &str) -> String {
        match active_language() {
            Language::Spanish => format!("revertido: {original_message}"),
            Language::French => format!("revert : {original_message}"),
            Language::Russian => format!("revert: {original_message}"),
            Language::Turkish => format!("revert edildi: {original_message}"),
            Language::English => format!("reverted: {original_message}"),
        }
    }

    pub fn rebasing_selected_commit() -> String {
        match active_language() {
            Language::Spanish => "Haciendo rebase de la rama actual sobre el commit seleccionado...".to_string(),
            Language::French => "Rebase de la branche actuelle sur le commit sélectionné...".to_string(),
            Language::Russian => "Rebase текущей ветки на выбранный коммит...".to_string(),
            Language::Turkish => "Geçerli dal seçili commit üzerine rebase ediliyor...".to_string(),
            Language::English => "Rebasing the current branch onto the selected commit...".to_string(),
        }
    }

    pub fn merging_selected_commit() -> String {
        match active_language() {
            Language::Spanish => "Haciendo merge del commit seleccionado en la rama actual...".to_string(),
            Language::French => "Merge du commit sélectionné dans la branche actuelle...".to_string(),
            Language::Russian => "Merge выбранного коммита в текущую ветку...".to_string(),
            Language::Turkish => "Seçili commit geçerli dala merge ediliyor...".to_string(),
            Language::English => "Merging the selected commit into the current branch...".to_string(),
        }
    }
}

localized_module!(settings {
    ACTIONS => " actions:",
    ACTIVE_CUSTOM => " active custom:",
    ACTIVE_CUSTOM_SYMBOLS => " active custom symbols:",
    AUTH => "auth",
    AUTHORIZATION => " authorization:",
    BRANCHES => "branches",
    COMMITTER_DATE_TIME => "committer date/time",
    COMMITTERS => "committers",
    CREDENTIALS => " credentials:",
    DEFAULT_REMOTE => " default remote:",
    DISPLAY => "display",
    EMAIL => " email:",
    ENTER_ACTION => "(enter)",
    GENERAL => "general",
    GRAPH_METADATA => " graph metadata:",
    GRAPH_LANE_LIMIT => " graph lane limit:",
    GRAPH_REFLOG_COMMITS => "graph reflog commits",
    HTTPS => " https:",
    HTTPS_DETAIL => "username/password or token prompt ",
    INSPECTOR => "inspector",
    KEYMAP => " keymap:",
    LANGUAGE => " language:",
    LAYOUT => " layout:",
    NAME => " name:",
    PANE_VISIBILITY => " pane visibility:",
    PERFORMANCE => " performance:",
    PATHS => "paths",
    PATHS_SECTION => " paths:",
    RECENT_FILE => " recent file:",
    RECENT_REPOSITORIES => " recent repositories:",
    REFLOG => "reflog",
    REFS => "refs",
    REMOTE_ERROR => " remote error:",
    REMOTES => " remotes:",
    REMOTES_ACTIONS_DETAIL => "select remote to manage | + add remote to create ",
    REPO => "repo",
    RESET_LAYOUT => "reset layout",
    SECRETS => " secrets:",
    SECRETS_DETAIL => "session only ",
    SETTINGS => "settings",
    SEARCH => "search",
    SHAS => "SHAs",
    SHORTCUTS => "shortcuts",
    SHORTCUTS_ACTION_MODE => " shortcuts / action mode:",
    SHORTCUTS_NORMAL_MODE => " shortcuts / normal mode:",
    SSH_FALLBACK => " ssh fallback:",
    SSH_FALLBACK_DETAIL => "key passphrase prompt ",
    SSH_AGENT_DETAIL => "ssh-agent when available ",
    STASHES => "stashes",
    STATUS => "status",
    SUBMODULES => "submodules",
    SYMBOLS => " symbols:",
    TAGS => "tags",
    THEME => " theme:",
    THEMES => " themes:",
    SYMBOL_THEME => " symbol theme:",
    SYMBOL_THEMES => " symbol themes:",
    VERSION => " version:",
    WORKTREES => "worktrees",
    ADD_REMOTE => " + add remote",
    FETCH_SUFFIX => "fetch:",
    PUSH_SUFFIX => "push:",
});

pub mod splash {
    use super::{Language, active_language, tr};

    localized_fns! {
    ACTIONS => "actions:",
    KEY_MOVE_DOWN_FALLBACK => "Shift + J",
    KEY_MOVE_UP_FALLBACK => "Shift + K",
    KEY_REMOVE_FALLBACK => "d",
    LOADING => "loading...",
    MADE_WITH => "made with ♡",
    MOVE_DOWN => "move down",
    MOVE_UP => "move up",
    NOT_A_VALID_GIT_REPOSITORY => "! not a valid git repository !",
    RECENT_REPOSITORIES => "recent repositories:",
    REMOVE => "remove",
    REPOSITORY_URL => "https://github.com/asinglebit/guitar",
    }

    pub fn recent_actions(remove: &str, move_up: &str, move_down: &str) -> String {
        match active_language() {
            Language::Spanish => format!("{} ({remove}) | {} ({move_up}) | {} ({move_down})", REMOVE(), MOVE_UP(), MOVE_DOWN()),
            Language::French => format!("{} ({remove}) | {} ({move_up}) | {} ({move_down})", REMOVE(), MOVE_UP(), MOVE_DOWN()),
            Language::Russian => format!("{} ({remove}) | {} ({move_up}) | {} ({move_down})", REMOVE(), MOVE_UP(), MOVE_DOWN()),
            Language::Turkish => format!("{} ({remove}) | {} ({move_up}) | {} ({move_down})", REMOVE(), MOVE_UP(), MOVE_DOWN()),
            Language::English => format!("{} ({remove}) | {} ({move_up}) | {} ({move_down})", REMOVE(), MOVE_UP(), MOVE_DOWN()),
        }
    }

    pub fn actions(text: &str) -> String {
        format!("{} {text}", ACTIONS())
    }
}

localized_module!(status {
    DETACHED => "detached",
    DETACHED_HEAD => "detached head:",
    GRAPH => "graph",
    INSPECTOR => "inspector",
    MODAL => "modal",
    NOT_INITIALIZED => "not initialized",
    NO_HEAD_NO_COMMITS => "no head (no commits yet)",
    SEARCH => "search",
    STAGED => "staged",
    STASH => "stash",
    UNSTAGED => "unstaged",
    VIEWER => "viewer",
    MODIFIED => "modified",
    NEW_COMMITS => "new commits",
    UNTRACKED => "untracked",
});

#[cfg(test)]
#[path = "../tests/helpers/localisation.rs"]
mod tests;
