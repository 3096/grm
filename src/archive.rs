use std::fs;
use std::io;
use std::path::Path;

pub struct ArchiveManager;

impl ArchiveManager {
    /// Extracts an archive file to a destination directory.
    /// Supports .zip and .tar.gz / .tgz
    pub fn extract(archive_path: &Path, destination: &Path) -> Result<(), String> {
        let extension = archive_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if extension == "zip" {
            Self::extract_zip(archive_path, destination)
        } else if extension == "gz" || archive_path.to_string_lossy().ends_with(".tgz") {
            Self::extract_tar_gz(archive_path, destination)
        } else {
            // Not an archive we know how to handle, just copy the file
            fs::copy(
                archive_path,
                destination.join(archive_path.file_name().unwrap()),
            )
            .map_err(|e| format!("Failed to copy non-archive file: {}", e))?;
            Ok(())
        }
    }

    fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), String> {
        let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let outpath = destination.join(file.name());

            if file.is_dir() {
                fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
                io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<(), String> {
        let tar_gz = fs::File::open(archive_path).map_err(|e| e.to_string())?;
        let tar_gz = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar_gz);

        archive.unpack(destination).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Handles the logic of moving extracted content to the final destination.
    /// 1. If extracted content is a single folder, merge its contents into the target directory.
    /// 2. If extracted content is multiple files/folders, move them into the target directory.
    pub fn finalize_extraction(
        temp_extract_dir: &Path,
        final_destination: &Path,
        repo_name: &str,
    ) -> Result<(), String> {
        let entries: Vec<_> = fs::read_dir(temp_extract_dir)
            .map_err(|e| format!("Failed to read temp extract dir: {}", e))?
            .filter_map(|res| res.ok())
            .collect();

        let target_dir = final_destination.join(repo_name);

        // Ensure parent directory exists
        if let Some(parent) = target_dir.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

        if entries.len() == 1 {
            let entry = &entries[0];
            if entry.path().is_dir() {
                // Case 1: Single folder inside -> merge its contents into target_dir
                for item in fs::read_dir(entry.path()).map_err(|e| e.to_string())? {
                    let item = item.map_err(|e| e.to_string())?;
                    Self::move_recursive(&item.path(), &target_dir.join(item.file_name()))?;
                }
                fs::remove_dir(entry.path()).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }

        // Case 2: Multiple files or a single file -> move them into target_dir
        for entry in entries {
            Self::move_recursive(&entry.path(), &target_dir.join(entry.file_name()))?;
        }

        Ok(())
    }

    fn move_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        if src.is_dir() {
            if dst.exists() && !dst.is_dir() {
                fs::remove_file(dst).map_err(|e| e.to_string())?;
            }
            if !dst.exists() {
                fs::create_dir_all(dst).map_err(|e| e.to_string())?;
            }

            for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                Self::move_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            }
            fs::remove_dir(src).map_err(|e| e.to_string())?;
        } else {
            // Overwrite file
            if let Err(e) = fs::rename(src, dst) {
                if e.raw_os_error() == Some(17) || e.raw_os_error() == Some(18) {
                    fs::copy(src, dst).map_err(|e| e.to_string())?;
                    fs::remove_file(src).map_err(|e| e.to_string())?;
                } else {
                    return Err(format!("Failed to move file: {}", e));
                }
            }
        }
        Ok(())
    }

    fn copy_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        if src.is_dir() {
            fs::create_dir_all(dst).map_err(|e| e.to_string())?;
            for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                Self::copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            }
        } else {
            fs::copy(src, dst).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_zip(path: &Path, files: &[(&str, &str)]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();

        for (name, content) in files {
            zip.start_file(name, options.clone()).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    fn create_test_tar_gz(path: &Path, files: &[(&str, &str)]) {
        let tar_gz = File::create(path).unwrap();
        let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
        let mut archive = tar::Builder::new(enc);

        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            archive
                .append_data(&mut header, name, content.as_bytes())
                .unwrap();
        }
        archive.finish().unwrap();
    }

    #[test]
    fn test_extract_zip() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("test.zip");
        let extract_path = dir.path().join("extracted");
        fs::create_dir_all(&extract_path).unwrap();

        create_test_zip(
            &archive_path,
            &[("hello.txt", "world"), ("folder/test.txt", "content")],
        );

        ArchiveManager::extract(&archive_path, &extract_path).expect("Zip extraction failed");

        assert!(extract_path.join("hello.txt").exists());
        assert!(extract_path.join("folder/test.txt").exists());
    }

    #[test]
    fn test_extract_tar_gz() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("test.tar.gz");
        let extract_path = dir.path().join("extracted");
        fs::create_dir_all(&extract_path).unwrap();

        create_test_tar_gz(
            &archive_path,
            &[("hello.txt", "world"), ("folder/test.txt", "content")],
        );

        ArchiveManager::extract(&archive_path, &extract_path).expect("Tar.gz extraction failed");

        assert!(extract_path.join("hello.txt").exists());
        assert!(extract_path.join("folder/test.txt").exists());
    }

    #[test]
    fn test_extract_non_archive() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "some content").unwrap();

        let extract_path = dir.path().join("extracted");
        fs::create_dir_all(&extract_path).unwrap();

        ArchiveManager::extract(&file_path, &extract_path).expect("Non-archive extraction failed");

        assert!(extract_path.join("test.txt").exists());
    }

    #[test]
    fn test_finalize_extraction_single_folder() {
        let dir = tempdir().unwrap();
        let temp_extract_dir = dir.path().join("extracted");
        let final_dest = dir.path().join("final");
        fs::create_dir_all(&temp_extract_dir).unwrap();

        let inner_folder = temp_extract_dir.join("my_tool_v1");
        fs::create_dir_all(&inner_folder).unwrap();
        fs::write(inner_folder.join("bin"), "data").unwrap();

        ArchiveManager::finalize_extraction(&temp_extract_dir, &final_dest, "my_tool")
            .expect("Finalize failed");

        assert!(final_dest.join("my_tool/bin").exists());
    }

    #[test]
    fn test_finalize_extraction_multiple_files() {
        let dir = tempdir().unwrap();
        let temp_extract_dir = dir.path().join("extracted");
        let final_dest = dir.path().join("final");
        fs::create_dir_all(&temp_extract_dir).unwrap();

        fs::write(temp_extract_dir.join("file1.txt"), "1").unwrap();
        fs::write(temp_extract_dir.join("file2.txt"), "2").unwrap();

        ArchiveManager::finalize_extraction(&temp_extract_dir, &final_dest, "my_tool")
            .expect("Finalize failed");

        assert!(final_dest.join("file1.txt").exists());
        assert!(final_dest.join("file2.txt").exists());
    }
}
