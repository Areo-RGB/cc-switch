//! DeepSeek Harness profile patch handling for MCP servers.
//! Profiles are shared with the native client and edited atomically.
//! Profiles share `$DSH_HOME`'s sessions, skills and `AGENTS.md`.
use crate::config::{atomic_write_private, get_home_dir};
use crate::error::AppError;
use serde_json::{json, Value};
use serde_yaml::Value as Yaml;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) fn data_dir() -> PathBuf {
    std::env::var_os("DSH_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| get_home_dir().join(".dsh"))
}

/// Profiles CC Switch writes, in this order. Web and Desktop are prepared before first launch.
const TARGET_PROFILES: [&str; 4] = ["web", "desktop", "headless", "acp"];
/// MCP entries are shown from the first of these that exists.
const DISPLAY_PROFILES: [&str; 2] = ["web", "desktop"];

pub(crate) fn profile_dir(name: &str) -> PathBuf {
    data_dir().join("profiles").join(name)
}

pub(crate) fn profile_path(name: &str) -> PathBuf {
    profile_dir(name).join("cordis.patch.yml")
}

/// Always prepare Web and Desktop: DSH's initProfile preserves an existing patch and supplies
/// the remaining files on first launch. Optional runtime profiles are only written if present.
///
/// A profile directory reached through a link is the same file as its target (symlink, Windows
/// junction, 8.3 short name or case-only alias). Listing both spellings made
/// [`edit_files_and_commit`] take the same `<file>.lock` twice, so every MCP write
/// failed with a bogus concurrency conflict (#9100). One file is listed once.
pub(crate) fn target_paths() -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    TARGET_PROFILES
        .iter()
        .filter_map(|name| {
            let path = profile_path(name);
            (DISPLAY_PROFILES.contains(name) || path.exists()).then_some(path)
        })
        .filter(|path| {
            let identity = path_identity(path);
            if seen.contains(&identity) {
                return false;
            }
            seen.push(identity);
            true
        })
        .collect()
}

/// Stable identity of one native file: its real parent directory plus the file name.
///
/// The file itself may not exist yet — the first write to a profile creates `cordis.patch.yml` —
/// but its parent always does, so resolving the parent is enough. When even the parent cannot be
/// resolved the path is kept as written: a directory that does not exist cannot be an alias of
/// another one.
fn path_identity(path: &Path) -> PathBuf {
    let Some(name) = path.file_name() else {
        return path.to_path_buf();
    };
    match path.parent().map(fs::canonicalize) {
        Some(Ok(parent)) => parent.join(name),
        _ => path.to_path_buf(),
    }
}

/// The authoritative profile MCP entries are read from first: `web`, else `desktop`, else
/// the first target. Writes go to every target, so [`read_paths`] also lists the rest.
pub(crate) fn config_path() -> PathBuf {
    DISPLAY_PROFILES
        .iter()
        .map(|name| profile_path(name))
        .find(|path| path.exists())
        .or_else(|| target_paths().into_iter().find(|path| path.exists()))
        .unwrap_or_else(|| profile_path("web"))
}

/// Every profile whose MCP entries are shown, authoritative profile first.
///
/// MCP entries are written to all initialized profiles, so a server that only exists in one
/// profile must still be listed.
pub(crate) fn read_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let authoritative = config_path();
    for path in std::iter::once(authoritative.clone()).chain(target_paths()) {
        if (path == authoritative || path.exists()) && !paths.contains(&path) {
            paths.push(path);
        }
    }
    if paths.is_empty() {
        paths.push(authoritative);
    }
    paths
}

/// Profile directory name of a `profiles/<name>/cordis.patch.yml` path, for error messages.
fn profile_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("profile")
        .to_owned()
}

/// `$DSH_HOME/cordis.patch.yml`, applied after every profile.
fn read_text(path: &Path) -> Result<String, AppError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(AppError::io(path, error)),
    }
}

pub(crate) fn entries(text: &str) -> Result<Vec<Yaml>, AppError> {
    let value: Yaml = serde_yaml::from_str(text)
        .map_err(|_| AppError::Config("Invalid DeepSeek Harness profile YAML".into()))?;
    match value {
        Yaml::Null => Ok(vec![]),
        Yaml::Sequence(entries) => Ok(entries),
        _ => Err(AppError::Config(
            "DeepSeek Harness profile must be a YAML sequence".into(),
        )),
    }
}

/// Edit a top-level patch, including an insert list without its own plugin id.
fn replace_patch(
    text: &str,
    parsed: &[Yaml],
    index: Option<usize>,
    entry: Option<Yaml>,
) -> Result<String, AppError> {
    let next = splice_patch(text, parsed, index, entry)?;
    // Removing or rewriting an entry can strand an alias whose anchor lived in it; refuse rather
    // than write a profile DSH can no longer load.
    entries(&next).map_err(|_| {
        AppError::Config(
            "This change would leave a YAML alias without its anchor; edit cordis.patch.yml directly".into(),
        )
    })?;
    Ok(next)
}

fn splice_patch(
    text: &str,
    parsed: &[Yaml],
    index: Option<usize>,
    entry: Option<Yaml>,
) -> Result<String, AppError> {
    // DSH requires a sequence even when the last patch is removed.
    let empty = if parsed.is_empty() || (parsed.len() == 1 && index.is_some()) {
        "[]\n"
    } else {
        ""
    };
    let reserialized = entry.is_some();
    let replacement = entry
        .map(|entry| serde_yaml::to_string(&vec![entry]))
        .transpose()
        .map_err(|error| AppError::Config(error.to_string()))?
        .unwrap_or_else(|| empty.to_owned());
    if parsed.is_empty() {
        // Native profiles start with comments followed by `[]`. Appending a
        // block entry to that flow sequence would produce invalid YAML.
        let mut next: String = text
            .split_inclusive('\n')
            .filter(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
            .collect();
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(&replacement);
        return Ok(next);
    }
    let mut starts = vec![];
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("- ") || line.trim_end() == "-" {
            starts.push(offset);
        }
        offset += line.len();
    }
    if parsed.len() != starts.len() {
        return Err(AppError::Config(
            "Use block-style YAML for the DeepSeek Harness profile".into(),
        ));
    }
    if let Some(index) = index {
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        // serde_yaml drops `!!js` tags and expands anchors, so rewriting an entry
        // that uses them would silently change what DSH evaluates.
        if reserialized
            && text[starts[index]..end]
                .lines()
                .any(crate::omp_config::has_yaml_reference_tokens)
        {
            return Err(AppError::Config(
                "This DeepSeek Harness profile entry uses YAML tags (such as !!js), anchors or aliases; edit it in cordis.patch.yml directly".into(),
            ));
        }
        Ok(format!(
            "{}{}{}",
            &text[..starts[index]],
            replacement,
            &text[end..]
        ))
    } else if replacement.is_empty() {
        Ok(text.into())
    } else {
        Ok(format!(
            "{}{}{}",
            text,
            if text.is_empty() || text.ends_with('\n') {
                ""
            } else {
                "\n"
            },
            replacement
        ))
    }
}

/// Same `<file>.lock` exclusive-create protocol and PID record as dsh-atomic-write.
struct FileLock(PathBuf);
impl FileLock {
    fn acquire(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
        }
        let lock = PathBuf::from(format!("{}.lock", path.display()));
        match Self::create(&lock) {
            Ok(guard) => Ok(guard),
            Err(AppError::Conflict(_)) if clear_stale_lock(&lock) => {
                // 上一次写入的进程已经不在了，锁是残留：清掉后重试一次。
                Self::create(&lock)
            }
            Err(error) => Err(error),
        }
    }

    fn create(lock: &Path) -> Result<Self, AppError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    AppError::Conflict(
                        "DeepSeek Harness is saving its configuration; retry shortly".into(),
                    )
                } else {
                    AppError::io(lock, error)
                }
            })?;
        let guard = Self(lock.to_path_buf());
        writeln!(file, "{}", std::process::id()).map_err(|error| AppError::io(&guard.0, error))?;
        Ok(guard)
    }
}
impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// 崩溃残留的 `<file>.lock` 会把之后的写入永久卡住（DSH 原生协议只做 create_new）。
/// 只有在能证明锁的主人已经不在时才删除：Unix 用 `kill(pid, 0)`、Windows 用 `OpenProcess` 判活，都用修改
/// 时间兜底（一次写入的锁不可能留 10 分钟）。
const STALE_LOCK_AGE: std::time::Duration = std::time::Duration::from_secs(10 * 60);

fn clear_stale_lock(lock: &Path) -> bool {
    let owner_gone = fs::read_to_string(lock)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .is_some_and(|pid| !process_is_alive(pid));
    let too_old = fs::metadata(lock)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > STALE_LOCK_AGE);
    (owner_gone || too_old) && fs::remove_file(lock).is_ok()
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    // SAFETY: kill(2) 只读内核状态；pid 0 代表进程组，已排除。
    pid > 0 && unsafe { libc::kill(pid as libc::pid_t, 0) } == 0
}

/// Windows：按 pid 打开进程；只有系统明确回答“没有这个进程”（`ERROR_INVALID_PARAMETER`）
/// 或进程已退出才算不在，拒绝访问等其它失败一律按活着处理，交给 mtime 兜底。
#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    if pid == 0 {
        return true;
    }
    // SAFETY: 只查询；句柄在下面关掉。
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        // SAFETY: 紧跟在失败的 OpenProcess 之后读取本线程的错误码。
        return unsafe { GetLastError() } != ERROR_INVALID_PARAMETER;
    }
    let mut exit_code = 0u32;
    // SAFETY: 句柄有效，输出参数指向本地变量。
    let queried = unsafe { GetExitCodeProcess(handle, &mut exit_code) } != 0;
    // SAFETY: OpenProcess 返回的句柄只关这一次。
    unsafe { CloseHandle(handle) };
    !queried || exit_code == STILL_ACTIVE as u32
}

#[cfg(not(any(unix, windows)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

fn restore(path: &Path, previous: Option<Vec<u8>>) -> Result<(), AppError> {
    match previous {
        Some(bytes) => atomic_write_private(path, &bytes),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::io(path, error)),
        },
    }
}

/// Edit several native files as one change: each file holds its own DSH `<file>.lock` through
/// the database commit. `plan` maps every file's current text to its next text, so an invalid
/// edit writes nothing; a failed write or commit restores every file already written.
fn edit_files_and_commit<T>(
    paths: &[PathBuf],
    plan: impl FnOnce(&[String]) -> Result<Vec<String>, AppError>,
    commit: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    // 固定加锁顺序（按路径排序），避免两个并发写入以不同顺序拿锁时互相等死；
    // 调用方传进来的顺序只决定 plan 的下标语义，这里按排序后的顺序加锁。
    let mut lock_order: Vec<usize> = (0..paths.len()).collect();
    lock_order.sort_by(|left, right| paths[*left].cmp(&paths[*right]));
    let mut _guards = Vec::with_capacity(paths.len());
    for index in &lock_order {
        _guards.push(FileLock::acquire(&paths[*index])?);
    }
    let mut previous = Vec::with_capacity(paths.len());
    let mut current = Vec::with_capacity(paths.len());
    for path in paths {
        previous.push(if path.exists() {
            Some(fs::read(path).map_err(|error| AppError::io(path, error))?)
        } else {
            None
        });
        current.push(read_text(path)?);
    }
    let next = plan(&current)?;
    // plan 少给或多给一个文件都意味着某一处改动会被静默丢掉：宁可整体失败。
    if next.len() != paths.len() {
        return Err(AppError::Message(format!(
            "DeepSeek Harness write plan returned {} file(s) for {} file(s); nothing was written",
            next.len(),
            paths.len()
        )));
    }
    let mut written = vec![];
    let result = paths
        .iter()
        .zip(&current)
        .zip(&next)
        .enumerate()
        .filter(|(_, ((_, current), next))| current != next)
        .try_for_each(|(index, ((path, _), next))| {
            atomic_write_private(path, next.as_bytes())?;
            written.push(index);
            Ok(())
        })
        .and_then(|()| commit());
    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            let failures: Vec<_> = written
                .into_iter()
                .rev()
                .filter_map(|index| {
                    restore(&paths[index], previous[index].take())
                        .err()
                        .map(|rollback| rollback.to_string())
                })
                .collect();
            if failures.is_empty() {
                Err(error)
            } else {
                Err(AppError::Message(format!(
                    "{error}; restoring DeepSeek Harness configuration failed: {}",
                    failures.join("; ")
                )))
            }
        }
    }
}

fn authoritative_index(paths: &[PathBuf]) -> usize {
    let authoritative = config_path();
    paths
        .iter()
        .position(|path| *path == authoritative)
        .unwrap_or(0)
}

fn existing_mcp_entry(text: &str, id: &str) -> Result<Option<Yaml>, AppError> {
    let all_entries = entries(text)?;
    let found = plugin_entries(&all_entries)
        .filter(|(_, _, entry)| {
            entry["name"].as_str() == Some("@deepseek-ai/dsh-mcp-client")
                && entry["config"]["serverName"].as_str() == Some(id)
        })
        .map(|(_, _, entry)| entry.clone())
        .next();
    Ok(found)
}

/// Compare existing copies, preferring the displayed profile when it has one. A missing copy
/// is expected when a profile was initialized later; only differing contents are a conflict.
fn ensure_mcp_copies_agree(
    paths: &[PathBuf],
    authoritative: usize,
    texts: &[String],
    id: &str,
) -> Result<Option<Yaml>, AppError> {
    let mut source = texts
        .get(authoritative)
        .map(|text| existing_mcp_entry(text, id))
        .transpose()?
        .flatten()
        .map(|entry| (authoritative, entry));
    for (index, text) in texts.iter().enumerate() {
        let Some(copy) = existing_mcp_entry(text, id)? else {
            continue;
        };
        if let Some((source_index, source)) = &source {
            if copy != *source {
                return Err(diverged_mcp_error(
                    id,
                    paths.get(index),
                    paths.get(*source_index),
                ));
            }
        } else {
            source = Some((index, copy));
        }
    }
    Ok(source.map(|(_, entry)| entry))
}

fn diverged_mcp_error(id: &str, other: Option<&PathBuf>, source: Option<&PathBuf>) -> AppError {
    let other = other.map(|path| profile_name(path)).unwrap_or_default();
    let source = source.map(|path| profile_name(path)).unwrap_or_default();
    AppError::localized(
        "dsh.mcp_diverged",
        format!(
            "MCP 服务器 '{id}' 在 profile '{other}' 里的配置与 '{source}' 不同；改写会覆盖你在 DeepSeek Harness 里对 '{other}' 做的修改。请先在 DeepSeek Harness 里把两者改成一致，或手动编辑 profiles/{other}/cordis.patch.yml。"
        ),
        format!(
            "MCP server '{id}' differs between the '{other}' and '{source}' profiles; writing would discard your changes in '{other}'. Make them identical in DeepSeek Harness, or edit profiles/{other}/cordis.patch.yml directly."
        ),
    )
}

pub(crate) fn mcp_entries() -> Result<Vec<Yaml>, AppError> {
    let authoritative = config_path();
    let mut merged: Vec<Yaml> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for path in read_paths() {
        let entries = match mcp_entries_from(&read_text(&path)?) {
            Ok(entries) => entries,
            // 权威 profile 读不出来时照旧报错；其余 profile 只跳过并记日志。
            Err(error) if path != authoritative => {
                log::warn!(
                    "跳过无法解析的 DeepSeek Harness profile {}: {error}",
                    path.display()
                );
                continue;
            }
            Err(error) => return Err(error),
        };
        for entry in entries {
            let name = entry["config"]["serverName"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{entry:?}"));
            if names.contains(&name) {
                continue;
            }
            names.push(name);
            merged.push(entry);
        }
    }
    Ok(merged)
}

fn mcp_entries_from(text: &str) -> Result<Vec<Yaml>, AppError> {
    Ok(plugin_entries(&entries(text)?)
        .filter(|(_, _, entry)| entry["name"].as_str() == Some("@deepseek-ai/dsh-mcp-client"))
        .map(|(_, _, entry)| entry.clone())
        .collect())
}

fn plugin_entries(patches: &[Yaml]) -> impl Iterator<Item = (usize, Option<usize>, &Yaml)> {
    patches.iter().enumerate().flat_map(|(index, patch)| {
        std::iter::once((index, None, patch)).chain(
            patch["insert"]
                .as_sequence()
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(insert_index, entry)| (index, Some(insert_index), entry)),
        )
    })
}

/// Write one MCP server into every target profile as one change ([`edit_files_and_commit`]).
pub(crate) fn sync_mcp_and_commit<T>(
    id: &str,
    spec: Option<&Value>,
    commit: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    sync_mcp_files_and_commit(&target_paths(), id, spec, commit)
}

fn sync_mcp_files_and_commit<T>(
    paths: &[PathBuf],
    id: &str,
    spec: Option<&Value>,
    commit: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let paths: Vec<_> = if spec.is_none() {
        paths.iter().filter(|path| path.exists()).cloned().collect()
    } else {
        paths.to_vec()
    };
    if paths.is_empty() {
        return commit();
    }
    if let Some(spec) = spec {
        validate_mcp_spec(id, spec)?;
    }
    edit_files_and_commit(
        &paths,
        |texts| {
            let authoritative = authoritative_index(&paths);
            let source = ensure_mcp_copies_agree(&paths, authoritative, texts, id)?;
            texts
                .iter()
                .map(|text| mcp_edit_with_source(text, id, spec, source.as_ref()))
                .collect()
        },
        commit,
    )
}

fn validate_mcp_spec(id: &str, spec: &Value) -> Result<(), AppError> {
    crate::mcp::validation::validate_server_spec(spec)?;
    if spec["type"] == "sse" {
        return Err(AppError::InvalidInput("DeepSeek Harness supports stdio and Streamable HTTP MCP; use an HTTP endpoint instead of SSE".into()));
    }
    if id.is_empty()
        || id.len() > 32
        || id
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
    {
        return Err(AppError::InvalidInput(
            "DeepSeek Harness MCP names must contain 1–32 letters, digits, '-' or '_'".into(),
        ));
    }
    Ok(())
}

/// A missing copy inherits native plugin IDs and extension fields from an existing copy, so
/// creating it does not manufacture a conflict on the next edit or deletion.
fn mcp_edit_with_source(
    text: &str,
    id: &str,
    spec: Option<&Value>,
    source: Option<&Yaml>,
) -> Result<String, AppError> {
    if let Some(source) = source.filter(|_| spec.is_some()) {
        if existing_mcp_entry(text, id)?.is_none() {
            let seeded = insert_mcp_entry(text, source)?;
            return mcp_edit(&seeded, id, spec);
        }
    }
    mcp_edit(text, id, spec)
}

fn insert_mcp_entry(text: &str, entry: &Yaml) -> Result<String, AppError> {
    // serde_yaml does not round-trip native !!js tags. Refuse dynamic nodes before copying.
    serde_json::to_value(entry).map_err(|_| {
        AppError::Config(
            "This DeepSeek Harness MCP entry contains dynamic YAML; edit it in DeepSeek Harness"
                .into(),
        )
    })?;
    let parsed = entries(text)?;
    if let Some(id) = entry["id"].as_str() {
        if plugin_entries(&parsed).any(|(_, _, entry)| entry["id"].as_str() == Some(id)) {
            return Err(AppError::Conflict(format!(
                "DeepSeek Harness plugin id '{id}' is already in use"
            )));
        }
    }
    let patch = Yaml::Mapping(
        [(Yaml::from("insert"), Yaml::Sequence(vec![entry.clone()]))]
            .into_iter()
            .collect(),
    );
    replace_patch(text, &parsed, None, Some(patch))
}

/// One profile's text with MCP server `id` added, replaced or (when `spec` is `None`) removed.
fn mcp_edit(text: &str, id: &str, spec: Option<&Value>) -> Result<String, AppError> {
    {
        let all_entries = entries(text)?;
        let matches: Vec<_> = plugin_entries(&all_entries)
            .filter(|(_, _, entry)| {
                entry["name"].as_str() == Some("@deepseek-ai/dsh-mcp-client")
                    && entry["config"]["serverName"].as_str() == Some(id)
            })
            .collect();
        if matches.len() > 1 {
            return Err(AppError::Config(format!(
                "Duplicate DeepSeek Harness MCP server '{id}'"
            )));
        }
        let location = matches.into_iter().next();
        let existing = location.map(|(_, _, entry)| entry.clone());
        if existing.is_none() && spec.is_none() {
            return Ok(text.to_owned());
        }
        let plugin_id = existing
            .as_ref()
            .and_then(|entry| entry["id"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("cc-switch-mcp-{id}"));
        if existing.is_some()
            && existing
                .as_ref()
                .and_then(|entry| entry["id"].as_str())
                .is_none()
        {
            return Err(AppError::Config(
                "Assign an id to this native MCP entry before editing it in CC Switch".into(),
            ));
        }
        let id_matches = plugin_entries(&all_entries)
            .filter(|(_, _, entry)| entry["id"].as_str() == Some(&plugin_id))
            .count();
        if id_matches > usize::from(existing.is_some()) {
            return Err(AppError::Conflict(format!(
                "DeepSeek Harness plugin id '{plugin_id}' is already in use"
            )));
        }
        let next = if let Some(spec) = spec {
            let mut entry = existing.unwrap_or(
                serde_yaml::to_value(
                    json!({"id":plugin_id,"name":"@deepseek-ai/dsh-mcp-client","config":{}}),
                )
                .unwrap(),
            );
            entry
                .as_mapping_mut()
                .unwrap()
                .remove(Yaml::from("disabled"));
            let config = entry["config"].as_mapping_mut().ok_or_else(|| {
                AppError::Config("Invalid DeepSeek Harness MCP configuration".into())
            })?;
            for field in [
                "command",
                "args",
                "env",
                "cwd",
                "url",
                "headers",
                "transport",
            ] {
                config.remove(Yaml::from(field));
            }
            for (key, value) in spec.as_object().unwrap() {
                if key != "type" {
                    config.insert(key.as_str().into(), serde_yaml::to_value(value).unwrap());
                }
            }
            config.insert("serverName".into(), id.into());
            config.insert(
                "transport".into(),
                if spec.get("url").is_some() {
                    "streamable-http"
                } else {
                    "stdio"
                }
                .into(),
            );
            Some(entry)
        } else {
            None
        };
        match location {
            Some((index, Some(insert_index), _)) => {
                let mut patch = all_entries[index].clone();
                let inserted = patch["insert"].as_sequence_mut().unwrap();
                if let Some(next) = next {
                    inserted[insert_index] = next;
                } else {
                    inserted.remove(insert_index);
                }
                let next = if inserted.is_empty() {
                    None
                } else {
                    Some(patch)
                };
                replace_patch(text, &all_entries, Some(index), next)
            }
            Some((index, None, _)) => replace_patch(text, &all_entries, Some(index), next),
            None => replace_patch(
                text,
                &all_entries,
                None,
                next.map(|entry| {
                    let mut patch = serde_yaml::Mapping::new();
                    patch.insert("insert".into(), Yaml::Sequence(vec![entry]));
                    Yaml::Mapping(patch)
                }),
            ),
        }
    }
}
