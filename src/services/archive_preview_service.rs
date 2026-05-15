//! ZIP 文件只读预览服务。

use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::config::operations;
use crate::db::repository::{file_repo, property_repo};
use crate::entities::{file, file_blob};
use crate::errors::{
    AsterError, MapAsterErr, Result, auth_forbidden_with_subcode, validation_error_with_subcode,
};
use crate::runtime::PrimaryAppState;
use crate::services::archive_service::zip_scan::{
    ZipScanEntryKind, ZipScanLimits, scan_zip_archive,
};
use crate::services::workspace_storage_service::WorkspaceStorageScope;
use crate::services::{share_service, workspace_storage_service};
use crate::types::EntityType;
use crate::utils::raii::TempFileGuard;

const CACHE_SCHEMA_VERSION: u32 = 1;
const FORMAT_ZIP: &str = "zip";
const CACHE_NAMESPACE: &str = "system.archive_preview";
const ZIP_MANIFEST_CACHE_NAME: &str = "zip_manifest.v1";
const ENTITY_PROPERTY_VALUE_MAX_BYTES: usize = 65_536;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ArchivePreviewManifest {
    pub schema_version: u32,
    pub format: String,
    pub source_blob_id: i64,
    pub source_hash: String,
    pub generated_at: String,
    pub entry_count: i64,
    pub file_count: i64,
    pub directory_count: i64,
    pub total_uncompressed_size: i64,
    pub truncated: bool,
    pub entries: Vec<ArchivePreviewEntry>,
}

pub(crate) fn manifest_etag_value(manifest: &ArchivePreviewManifest) -> Result<String> {
    let value = serde_json::json!({
        "schema_version": manifest.schema_version,
        "format": manifest.format,
        "source_blob_id": manifest.source_blob_id,
        "source_hash": manifest.source_hash,
        "entry_count": manifest.entry_count,
        "file_count": manifest.file_count,
        "directory_count": manifest.directory_count,
        "total_uncompressed_size": manifest.total_uncompressed_size,
        "truncated": manifest.truncated,
        "entries": manifest.entries,
    });
    let bytes = serde_json::to_vec(&value).map_aster_err_ctx(
        "serialize archive preview manifest etag",
        AsterError::internal_error,
    )?;
    Ok(crate::utils::hash::sha256_hex(&bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ArchivePreviewEntry {
    pub path: String,
    pub name: String,
    pub parent: Option<String>,
    pub kind: ArchivePreviewEntryKind,
    pub size: i64,
    pub compressed_size: i64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub enum ArchivePreviewEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedArchivePreviewManifest {
    schema_version: u32,
    source_blob_id: i64,
    source_hash: String,
    limit_signature: String,
    manifest: ArchivePreviewManifest,
}

#[derive(Debug, Clone)]
struct ArchivePreviewLimits {
    max_source_bytes: i64,
    max_manifest_bytes: usize,
    max_duration_secs: u64,
    scan_limits: ZipScanLimits,
    signature: String,
}

pub(crate) async fn preview_file_in_scope(
    state: &PrimaryAppState,
    scope: WorkspaceStorageScope,
    file_id: i64,
) -> Result<ArchivePreviewManifest> {
    ensure_user_preview_enabled(state)?;
    workspace_storage_service::require_scope_access(state, scope).await?;
    let source_file = workspace_storage_service::verify_file_access(state, scope, file_id).await?;
    workspace_storage_service::ensure_active_file_scope(&source_file, scope)?;
    preview_verified_file(state, &source_file).await
}

pub(crate) async fn preview_shared_file(
    state: &PrimaryAppState,
    token: &str,
) -> Result<ArchivePreviewManifest> {
    ensure_share_preview_enabled(state)?;
    let (_, source_file) = share_service::load_preview_shared_file(state, token).await?;
    preview_verified_file(state, &source_file).await
}

pub(crate) async fn preview_shared_folder_file(
    state: &PrimaryAppState,
    token: &str,
    file_id: i64,
) -> Result<ArchivePreviewManifest> {
    ensure_share_preview_enabled(state)?;
    let (_, source_file) =
        share_service::load_preview_shared_folder_file(state, token, file_id).await?;
    preview_verified_file(state, &source_file).await
}

fn ensure_user_preview_enabled(state: &PrimaryAppState) -> Result<()> {
    ensure_preview_master_enabled(state)?;
    if !operations::archive_preview_user_enabled(&state.runtime_config) {
        return Err(auth_forbidden_with_subcode(
            "archive_preview.user_disabled",
            "archive preview for user files is disabled",
        ));
    }
    Ok(())
}

fn ensure_share_preview_enabled(state: &PrimaryAppState) -> Result<()> {
    ensure_preview_master_enabled(state)?;
    if !operations::archive_preview_share_enabled(&state.runtime_config) {
        return Err(auth_forbidden_with_subcode(
            "archive_preview.share_disabled",
            "archive preview for shared files is disabled",
        ));
    }
    Ok(())
}

fn ensure_preview_master_enabled(state: &PrimaryAppState) -> Result<()> {
    if !operations::archive_preview_enabled(&state.runtime_config) {
        return Err(auth_forbidden_with_subcode(
            "archive_preview.disabled",
            "archive preview is disabled",
        ));
    }
    Ok(())
}

async fn preview_verified_file(
    state: &PrimaryAppState,
    source_file: &file::Model,
) -> Result<ArchivePreviewManifest> {
    ensure_archive_preview_source_supported(source_file)?;
    let limits = ArchivePreviewLimits::from_runtime_config(&state.runtime_config)?;
    if source_file.size > limits.max_source_bytes {
        return Err(archive_preview_validation_error(
            "archive_preview.source_too_large",
            format!(
                "source archive size {} exceeds archive preview limit {}",
                source_file.size, limits.max_source_bytes
            ),
        ));
    }

    let blob = file_repo::find_blob_by_id(&state.db, source_file.blob_id).await?;
    if let Some(cached) = load_cached_manifest(state, source_file, &blob, &limits).await? {
        return Ok(cached);
    }

    let temp_path = prepare_archive_preview_temp_path(state).await?;
    let temp_guard = TempFileGuard::new(temp_path, "archive preview source temp file");
    download_blob_to_temp(state, source_file, &blob, temp_guard.path()).await?;
    let manifest = scan_manifest_from_temp(source_file, &blob, temp_guard.path(), &limits).await?;
    store_cached_manifest(state, source_file, &blob, &limits, &manifest).await;
    Ok(manifest)
}

impl ArchivePreviewLimits {
    fn from_runtime_config(runtime_config: &crate::config::RuntimeConfig) -> Result<Self> {
        let preview_max_entries = operations::archive_preview_max_entries(runtime_config);
        let scan_limits = ZipScanLimits {
            max_uncompressed_bytes: operations::archive_extract_max_uncompressed_bytes(
                runtime_config,
            ),
            max_entries: preview_max_entries
                .min(operations::archive_extract_max_entries(runtime_config)),
            max_files: preview_max_entries
                .min(operations::archive_extract_max_files(runtime_config)),
            max_directories: preview_max_entries
                .min(operations::archive_extract_max_directories(runtime_config)),
            max_depth: operations::archive_extract_max_depth(runtime_config),
            max_path_bytes: operations::archive_extract_max_path_bytes(runtime_config),
            max_compression_ratio: operations::archive_extract_max_compression_ratio(
                runtime_config,
            ),
            max_entry_compression_ratio: operations::archive_extract_max_entry_compression_ratio(
                runtime_config,
            ),
        };
        let max_manifest_bytes = crate::utils::numbers::u64_to_usize(
            operations::archive_preview_max_manifest_bytes(runtime_config),
            operations::ARCHIVE_PREVIEW_MAX_MANIFEST_BYTES_KEY,
        )?;
        let max_source_bytes = operations::archive_preview_max_source_bytes(runtime_config);
        let signature = format!(
            "source={};manifest={};entries={};files={};dirs={};uncompressed={};depth={};path={};ratio={};entry_ratio={}",
            max_source_bytes,
            max_manifest_bytes,
            scan_limits.max_entries,
            scan_limits.max_files,
            scan_limits.max_directories,
            scan_limits.max_uncompressed_bytes,
            scan_limits.max_depth,
            scan_limits.max_path_bytes,
            scan_limits.max_compression_ratio,
            scan_limits.max_entry_compression_ratio
        );

        Ok(Self {
            max_source_bytes,
            max_manifest_bytes,
            max_duration_secs: operations::archive_preview_max_duration_secs(runtime_config),
            scan_limits,
            signature,
        })
    }
}

async fn load_cached_manifest(
    state: &PrimaryAppState,
    source_file: &file::Model,
    blob: &file_blob::Model,
    limits: &ArchivePreviewLimits,
) -> Result<Option<ArchivePreviewManifest>> {
    let Some(prop) = property_repo::find_by_key(
        &state.db,
        EntityType::File,
        source_file.id,
        CACHE_NAMESPACE,
        ZIP_MANIFEST_CACHE_NAME,
    )
    .await?
    else {
        return Ok(None);
    };

    let Some(value) = prop.value else {
        return Ok(None);
    };
    let cached = match serde_json::from_str::<CachedArchivePreviewManifest>(&value) {
        Ok(cached) => cached,
        Err(error) => {
            tracing::warn!(
                file_id = source_file.id,
                property_id = prop.id,
                "failed to parse archive preview cache: {error}"
            );
            return Ok(None);
        }
    };

    if cached.schema_version == CACHE_SCHEMA_VERSION
        && cached.source_blob_id == blob.id
        && cached.source_hash == blob.hash
        && cached.limit_signature == limits.signature
        && cached.manifest.schema_version == CACHE_SCHEMA_VERSION
        && cached.manifest.format == FORMAT_ZIP
    {
        return Ok(Some(cached.manifest));
    }

    Ok(None)
}

async fn store_cached_manifest(
    state: &PrimaryAppState,
    source_file: &file::Model,
    blob: &file_blob::Model,
    limits: &ArchivePreviewLimits,
    manifest: &ArchivePreviewManifest,
) {
    let cached = CachedArchivePreviewManifest {
        schema_version: CACHE_SCHEMA_VERSION,
        source_blob_id: blob.id,
        source_hash: blob.hash.clone(),
        limit_signature: limits.signature.clone(),
        manifest: manifest.clone(),
    };
    let serialized = match serde_json::to_string(&cached) {
        Ok(serialized) => serialized,
        Err(error) => {
            tracing::warn!(
                file_id = source_file.id,
                "failed to serialize archive preview cache: {error}"
            );
            return;
        }
    };
    if serialized.len() > ENTITY_PROPERTY_VALUE_MAX_BYTES {
        tracing::debug!(
            file_id = source_file.id,
            bytes = serialized.len(),
            "archive preview cache exceeds entity property value limit; skipping cache write"
        );
        return;
    }

    if let Err(error) = property_repo::upsert(
        &state.db,
        EntityType::File,
        source_file.id,
        CACHE_NAMESPACE,
        ZIP_MANIFEST_CACHE_NAME,
        Some(&serialized),
    )
    .await
    {
        tracing::warn!(
            file_id = source_file.id,
            "failed to write archive preview cache: {error}"
        );
    }
}

async fn prepare_archive_preview_temp_path(state: &PrimaryAppState) -> Result<PathBuf> {
    let runtime_temp_dir = crate::utils::paths::runtime_temp_dir(&state.config.server.temp_dir);
    tokio::fs::create_dir_all(&runtime_temp_dir)
        .await
        .map_aster_err_ctx(
            "create archive preview temp dir",
            AsterError::storage_driver_error,
        )?;
    Ok(PathBuf::from(crate::utils::paths::runtime_temp_file_path(
        &state.config.server.temp_dir,
        &format!("archive-preview-{}.zip", uuid::Uuid::new_v4()),
    )))
}

async fn download_blob_to_temp(
    state: &PrimaryAppState,
    source_file: &file::Model,
    blob: &file_blob::Model,
    temp_path: &Path,
) -> Result<()> {
    let policy = state.policy_snapshot.get_policy_or_err(blob.policy_id)?;
    let driver = state.driver_registry.get_driver(&policy)?;
    let mut stream = driver.get_stream(&blob.storage_path).await?;
    let mut output = tokio::fs::File::create(temp_path).await.map_aster_err_ctx(
        "create archive preview source temp file",
        AsterError::storage_driver_error,
    )?;
    copy_async_reader_to_writer_with_expected_size(
        &mut stream,
        &mut output,
        crate::utils::numbers::i64_to_u64(source_file.size, "source archive size")?,
        "source archive",
    )
    .await?;
    output.flush().await.map_aster_err_ctx(
        "flush archive preview source temp file",
        AsterError::storage_driver_error,
    )?;
    Ok(())
}

async fn scan_manifest_from_temp(
    source_file: &file::Model,
    blob: &file_blob::Model,
    temp_path: &Path,
    limits: &ArchivePreviewLimits,
) -> Result<ArchivePreviewManifest> {
    let source_file_id = source_file.id;
    let source_blob_id = blob.id;
    let source_hash = blob.hash.clone();
    let path = temp_path.to_path_buf();
    let scan_limits = limits.scan_limits;
    let deadline =
        Instant::now().checked_add(std::time::Duration::from_secs(limits.max_duration_secs));
    let generated_at = Utc::now().to_rfc3339();

    let manifest = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&path).map_aster_err_ctx(
            "open archive preview temp file",
            AsterError::storage_driver_error,
        )?;
        let mut archive = zip::ZipArchive::new(file).map_aster_err_with(|| {
            archive_preview_validation_error("archive_preview.invalid_zip", "invalid zip archive")
        })?;
        let scanned = scan_zip_archive(&mut archive, scan_limits, deadline, |_| Ok(()))
            .map_err(map_archive_preview_scan_error)?;
        let entries = scanned
            .entries
            .into_iter()
            .map(|entry| ArchivePreviewEntry {
                path: entry.path,
                name: entry.name,
                parent: entry.parent,
                kind: match entry.kind {
                    ZipScanEntryKind::File => ArchivePreviewEntryKind::File,
                    ZipScanEntryKind::Directory => ArchivePreviewEntryKind::Directory,
                },
                size: entry.size,
                compressed_size: entry.compressed_size,
                modified_at: entry.modified_at,
            })
            .collect();

        Ok::<_, AsterError>(ArchivePreviewManifest {
            schema_version: CACHE_SCHEMA_VERSION,
            format: FORMAT_ZIP.to_string(),
            source_blob_id,
            source_hash,
            generated_at,
            entry_count: crate::utils::numbers::u64_to_i64(
                scanned.entry_count,
                "archive preview entry count",
            )?,
            file_count: crate::utils::numbers::u64_to_i64(
                scanned.file_count,
                "archive preview file count",
            )?,
            directory_count: crate::utils::numbers::u64_to_i64(
                scanned.directory_count,
                "archive preview directory count",
            )?,
            total_uncompressed_size: scanned.total_uncompressed_bytes,
            truncated: false,
            entries,
        })
    })
    .await
    .map_err(|error| {
        AsterError::internal_error(format!("archive preview worker failed: {error}"))
    })??;

    fit_manifest_to_limit(source_file_id, manifest, limits.max_manifest_bytes)
}

fn fit_manifest_to_limit(
    file_id: i64,
    manifest: ArchivePreviewManifest,
    max_manifest_bytes: usize,
) -> Result<ArchivePreviewManifest> {
    if serialized_manifest_len(&manifest)? <= max_manifest_bytes {
        return Ok(manifest);
    }

    let original_entries = manifest.entries.clone();
    let mut low = 0_usize;
    let mut high = original_entries.len();
    let mut best = None;

    while low <= high {
        let mid = low + (high - low) / 2;
        let mut candidate = manifest.clone();
        candidate.truncated = true;
        candidate.entries = original_entries[..mid].to_vec();

        if serialized_manifest_len(&candidate)? <= max_manifest_bytes {
            best = Some(candidate);
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }

    best.ok_or_else(|| {
        archive_preview_validation_error(
            "archive_preview.manifest_too_large",
            format!(
                "archive preview manifest for file #{file_id} exceeds server limit {max_manifest_bytes} bytes"
            ),
        )
    })
}

fn serialized_manifest_len(manifest: &ArchivePreviewManifest) -> Result<usize> {
    serde_json::to_vec(manifest)
        .map(|bytes| bytes.len())
        .map_aster_err_ctx(
            "serialize archive preview manifest",
            AsterError::internal_error,
        )
}

fn ensure_archive_preview_source_supported(source_file: &file::Model) -> Result<()> {
    let mime = source_file.mime_type.to_ascii_lowercase();
    if source_file.name.to_ascii_lowercase().ends_with(".zip")
        || matches!(
            mime.as_str(),
            "application/zip" | "application/x-zip-compressed"
        )
    {
        Ok(())
    } else {
        Err(archive_preview_validation_error(
            "archive_preview.unsupported_type",
            "archive preview currently supports .zip files only",
        ))
    }
}

fn archive_preview_validation_error(subcode: &str, message: impl Into<String>) -> AsterError {
    validation_error_with_subcode(subcode, message)
}

fn map_archive_preview_scan_error(error: AsterError) -> AsterError {
    if matches!(error, AsterError::ValidationError(_)) && error.api_error_subcode().is_none() {
        return archive_preview_validation_error(
            "archive_preview.rejected",
            error.message().to_string(),
        );
    }
    error
}

async fn copy_async_reader_to_writer_with_expected_size<R, W>(
    reader: &mut R,
    writer: &mut W,
    expected_bytes: u64,
    context: &str,
) -> Result<u64>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin,
{
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer).await.map_aster_err_ctx(
            "read bounded archive preview stream chunk",
            AsterError::storage_driver_error,
        )?;
        if read == 0 {
            break;
        }

        let read_u64 =
            crate::utils::numbers::usize_to_u64(read, "archive preview stream chunk size")?;
        let next_copied = copied.checked_add(read_u64).ok_or_else(|| {
            AsterError::internal_error("archive preview stream byte counter overflow")
        })?;
        if next_copied > expected_bytes {
            return Err(archive_preview_validation_error(
                "archive_preview.source_size_mismatch",
                format!("{context} expands beyond declared size: declared {expected_bytes} bytes"),
            ));
        }

        writer.write_all(&buffer[..read]).await.map_aster_err_ctx(
            "write bounded archive preview stream chunk",
            AsterError::storage_driver_error,
        )?;
        copied = next_copied;
    }

    if copied != expected_bytes {
        return Err(archive_preview_validation_error(
            "archive_preview.source_size_mismatch",
            format!(
                "{context} size mismatch: declared {expected_bytes} bytes, downloaded {copied} bytes"
            ),
        ));
    }

    Ok(copied)
}
