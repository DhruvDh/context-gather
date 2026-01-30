use crate::chunker;
use crate::context::gather;
use crate::context::types::FileContents;
use crate::header;
use crate::output::{self, RenderedChunk};
use crate::xml_output;
use anyhow::{Result, anyhow};
use globset::{Glob, GlobSetBuilder};
use path_slash::{PathBufExt, PathExt};
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug)]
pub struct InvalidExcludePatterns {
    pub patterns: Vec<String>,
}

impl std::fmt::Display for InvalidExcludePatterns {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "Every --exclude pattern was invalid: {:?}",
            self.patterns
        )
    }
}

impl std::error::Error for InvalidExcludePatterns {}

/// Pipeline for gathering and assembling context outputs.
#[derive(Default)]
pub struct Pipeline {
    root: PathBuf,
    user_paths_raw: Vec<PathBuf>,
    user_paths_canon: Vec<PathBuf>,
    candidate_files: Vec<PathBuf>,
    preselected_paths: Vec<PathBuf>,
    file_data: Vec<FileContents>,
    xml_output: Option<String>,
    rendered_chunks: Vec<RenderedChunk>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Expand user-provided paths and cache canonical versions for preselection.
    pub fn expand_paths(
        &mut self,
        paths: &[String],
    ) -> Result<()> {
        let root = std::env::current_dir()?;
        self.root = dunce::canonicalize(root)?;
        self.user_paths_raw = gather::expand_paths(paths.to_vec())?;
        self.user_paths_canon = self
            .user_paths_raw
            .iter()
            .filter_map(|p| dunce::canonicalize(p).ok())
            .collect();
        Ok(())
    }

    /// Build candidate file list (explicit files + files under directories).
    pub fn build_candidates(&mut self) -> Result<()> {
        let mut candidate_files: Vec<PathBuf> = Vec::new();
        let mut dirs_to_scan: Vec<PathBuf> = Vec::new();
        for up in &self.user_paths_raw {
            if up.is_dir() {
                dirs_to_scan.push(up.clone());
            } else {
                candidate_files.push(up.clone());
            }
        }
        if !dirs_to_scan.is_empty() {
            candidate_files.extend(gather::gather_all_file_paths(&dirs_to_scan)?);
        }

        // Canonicalize and deduplicate explicit and discovered files
        for path in &mut candidate_files {
            if let Ok(canon) = dunce::canonicalize(&*path) {
                *path = canon;
            }
        }
        candidate_files.sort();
        candidate_files.dedup();

        self.candidate_files = candidate_files;
        Ok(())
    }

    /// Compute which candidates are preselected (under user paths).
    pub fn compute_preselected(&mut self) {
        self.preselected_paths = self
            .candidate_files
            .iter()
            .filter(|cand| is_preselected(cand, &self.user_paths_canon))
            .cloned()
            .collect();
    }

    pub fn set_candidate_files(
        &mut self,
        files: Vec<PathBuf>,
    ) {
        self.candidate_files = files;
    }

    pub fn candidate_files(&self) -> &[PathBuf] {
        &self.candidate_files
    }

    pub fn preselected_paths(&self) -> &[PathBuf] {
        &self.preselected_paths
    }

    pub fn file_data(&self) -> &[FileContents] {
        &self.file_data
    }

    pub fn xml_output(&self) -> Option<&str> {
        self.xml_output.as_deref()
    }

    pub fn rendered_chunks(&self) -> &[RenderedChunk] {
        &self.rendered_chunks
    }

    /// Apply exclude patterns to candidate files.
    pub fn apply_excludes(
        &mut self,
        exclude: &[String],
    ) -> Result<()> {
        let raw_patterns: Vec<String> = exclude.iter().map(|p| p.replace('\\', "/")).collect();
        let mut builder = GlobSetBuilder::new();
        let mut valid = 0usize;
        for pattern in &raw_patterns {
            match Glob::new(pattern) {
                Ok(glob) => {
                    builder.add(glob);
                    valid += 1;
                }
                Err(err) => {
                    warn!("invalid --exclude-paths pattern: {pattern} ({err})");
                }
            }
        }
        if !raw_patterns.is_empty() && valid == 0 {
            return Err(anyhow!(InvalidExcludePatterns {
                patterns: raw_patterns,
            }));
        }
        if valid == 0 {
            return Ok(());
        }

        let matcher = builder.build()?;
        self.candidate_files.retain(|path| {
            let abs = path.to_slash_lossy();
            let rel = path
                .strip_prefix(&self.root)
                .ok()
                .map(|p| p.to_slash_lossy());
            let rel = rel.as_deref().unwrap_or(abs.as_ref());
            !matcher.is_match(rel) && !matcher.is_match(abs.as_ref())
        });
        Ok(())
    }

    /// Read file data into memory.
    pub fn collect_file_data(
        &mut self,
        max_size: u64,
    ) -> Result<()> {
        self.file_data = gather::collect_file_data(&self.candidate_files, max_size, &self.root)?;
        Ok(())
    }

    /// Build the full XML output (folder-grouped) for non-chunked mode.
    pub fn build_xml(
        &mut self,
        escape_xml: bool,
    ) -> Result<()> {
        self.xml_output = Some(xml_output::build_xml_with_escape(
            &self.file_data,
            escape_xml,
        )?);
        Ok(())
    }

    /// Build chunked output with header (for chunked/multi-step modes).
    pub fn build_chunks_with_header(
        &mut self,
        chunk_limit: usize,
        escape_xml: bool,
        multi_step: bool,
        include_git: bool,
    ) -> Result<()> {
        if multi_step {
            let metas = chunker::build_file_meta(&self.file_data, escape_xml);
            let header_xml = format!(
                "<shared-context>\n{}\n",
                header::make_header(1, chunk_limit, &metas, multi_step, escape_xml, include_git,)
            );
            let header_tokens = gather::count_tokens(&header_xml);
            self.rendered_chunks = vec![RenderedChunk {
                tokens: header_tokens,
                xml: header_xml,
            }];
            return Ok(());
        }

        let mut effective_limit = chunk_limit;
        for attempt in 0..8 {
            let (mut bodies, metas) =
                chunker::build_chunk_bodies(&self.file_data, effective_limit, escape_xml);
            let max_blocks: usize = bodies.iter().map(|b| b.blocks.len()).sum();
            let mut splits = 0usize;
            let mut header_oversize = false;
            loop {
                let total_chunks = bodies.len() + 1;
                let header_xml = format!(
                    "<shared-context>\n{}\n",
                    header::make_header(
                        total_chunks,
                        chunk_limit,
                        &metas,
                        multi_step,
                        escape_xml,
                        include_git,
                    )
                );
                let wrapper_floor = if chunk_limit > 0 {
                    let last_id = total_chunks.saturating_sub(1);
                    let wrapper = format!(
                        "<context-chunk id=\"{}/{}\">\n</context-chunk>\n</shared-context>\n",
                        last_id, total_chunks
                    );
                    let wrapper_tokens = gather::count_tokens(&wrapper);
                    chunk_limit.saturating_sub(wrapper_tokens.saturating_add(2))
                } else {
                    0
                };
                let body_xmls: Vec<String> = bodies
                    .iter()
                    .map(|body| body.blocks.iter().map(|b| b.xml.as_str()).collect())
                    .collect();
                let mut snippet_tokens = Vec::with_capacity(total_chunks);
                let mut snippet_xmls = Vec::with_capacity(total_chunks);
                let mut split_body_idx = None;
                let mut oversize_single = Vec::new();
                let mut required_limit: Option<usize> = None;
                let mut max_over_limit = 0usize;
                let mut has_unavoidable = false;
                for idx in 0..total_chunks {
                    let snippet = output::render_chunk_snippet(&header_xml, &body_xmls, idx);
                    let tokens = gather::count_tokens(&snippet);
                    snippet_tokens.push(tokens);
                    snippet_xmls.push(snippet);
                    if chunk_limit > 0 && tokens > chunk_limit {
                        if idx == 0 {
                            header_oversize = true;
                        } else {
                            let body_idx = idx - 1;
                            if bodies[body_idx].blocks.len() > 1 {
                                split_body_idx = Some(body_idx);
                                break;
                            } else {
                                oversize_single.push(idx);
                                let block_tokens = bodies[body_idx].blocks[0].tokens;
                                if block_tokens > chunk_limit {
                                    has_unavoidable = true;
                                    continue;
                                }
                                let overhead = tokens.saturating_sub(block_tokens);
                                let limit = chunk_limit.saturating_sub(overhead).max(wrapper_floor);
                                required_limit = Some(match required_limit {
                                    Some(prev) => prev.min(limit),
                                    None => limit,
                                });
                            }
                            max_over_limit = max_over_limit.max(tokens.saturating_sub(chunk_limit));
                        }
                    }
                }

                if let Some(body_idx) = split_body_idx {
                    let last_block = bodies[body_idx]
                        .blocks
                        .pop()
                        .expect("chunk should contain at least one block");
                    bodies[body_idx].tokens =
                        bodies[body_idx].tokens.saturating_sub(last_block.tokens);
                    let last_tokens = last_block.tokens;
                    bodies.insert(
                        body_idx + 1,
                        chunker::ChunkBody {
                            blocks: vec![last_block],
                            tokens: last_tokens,
                        },
                    );
                    splits += 1;
                    if splits > max_blocks {
                        return Err(anyhow!("chunk splitting did not converge"));
                    }
                    continue;
                }

                if let Some(limit) = required_limit
                    && limit > 0
                    && limit < effective_limit
                {
                    let adjusted = limit.saturating_sub(2).max(1);
                    if adjusted < effective_limit {
                        effective_limit = adjusted;
                        break;
                    }
                    effective_limit = limit;
                    break;
                }
                if max_over_limit > 0
                    && !has_unavoidable
                    && effective_limit > wrapper_floor.saturating_add(1)
                {
                    effective_limit = effective_limit.saturating_sub(1).max(wrapper_floor);
                    break;
                }

                if header_oversize {
                    warn!(
                        "header exceeds chunk size {}; increase --chunk-size or disable git info",
                        chunk_limit
                    );
                }
                if !oversize_single.is_empty() {
                    warn!(
                        "one or more chunks exceed the chunk size {} due to oversize file parts",
                        chunk_limit
                    );
                }

                self.rendered_chunks = snippet_xmls
                    .into_iter()
                    .zip(snippet_tokens)
                    .map(|(xml, tokens)| RenderedChunk { xml, tokens })
                    .collect();
                return Ok(());
            }
            if attempt == 7 {
                return Err(anyhow!("chunk splitting did not converge"));
            }
        }
        Err(anyhow!("chunk splitting did not converge"))
    }
}

// Helper: check if `candidate` is "under" any user-specified path (including exact matches).
fn is_preselected(
    candidate: &Path,
    user_paths: &[PathBuf],
) -> bool {
    user_paths.iter().any(|up| candidate.starts_with(up))
}
