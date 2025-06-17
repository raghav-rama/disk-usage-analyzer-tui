use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use indicatif::ProgressBar;
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct DirEntryInfo {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<DirEntryInfo>,
}

pub fn build_tree(root: &Path, follow_symlinks: bool, _pb: &ProgressBar) -> std::io::Result<DirEntryInfo> {
    let mut entries: Vec<(PathBuf, u64, bool)> = WalkBuilder::new(root)
        .follow_links(follow_symlinks)
        .hidden(false)
        .threads(num_cpus::get())
        .build()
        .par_bridge()
        .filter_map(|entry| match entry {
            Ok(dirent) => {
                if dirent.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    let md = dirent.metadata().ok()?;
                    let sz = md.len();
                    Some((dirent.into_path(), sz, false))
                } else {
                    Some((dirent.into_path(), 0, true))
                }
            }
            Err(_) => None,
        })
        .collect();

    entries.sort_by_key(|(p, _, _)| p.clone());

    use std::collections::HashMap;
    let mut sizes: HashMap<PathBuf, u64> = HashMap::new();
    for (path, size, _) in &entries {
        sizes.entry(path.clone()).or_default();
        if *size > 0 {
            sizes.entry(path.clone()).and_modify(|s| *s += *size);
        }
        let mut cur = path.parent();
        while let Some(p) = cur {
            sizes.entry(p.to_path_buf()).or_default();
            sizes.entry(p.to_path_buf()).and_modify(|s| *s += *size);
            cur = p.parent();
        }
    }

    fn build_node(
        path: &Path,
        sizes: &HashMap<PathBuf, u64>,
        is_dir: bool,
        entries: &[(PathBuf, u64, bool)],
    ) -> DirEntryInfo {
        let children_paths: Vec<&(PathBuf, u64, bool)> = entries
            .iter()
            .filter(|(p, _, _)| p.parent() == Some(path))
            .collect();
        let children = children_paths
            .iter()
            .map(|(p, _, isd)| build_node(p, sizes, *isd, entries))
            .collect();
        DirEntryInfo {
            path: path.to_path_buf(),
            size: *sizes.get(path).unwrap_or(&0),
            is_dir,
            children,
        }
    }

    let root_node = build_node(root, &sizes, true, &entries);
    Ok(root_node)
}
