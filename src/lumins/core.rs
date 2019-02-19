use rayon::prelude::*;
use std::io;

use crate::lumins::file_ops;
use crate::lumins::parse;

/// Synchronizes all files, directories, and symlinks in `dest` with `src`
///
/// # Arguments
/// * `src`: Source directory
/// * `dest`: Destination directory
///
/// # Errors
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
/// * `src` is an invalid directory
/// * `dest` is an invalid directory
pub fn synchronize(src: &str, dest: &str, flags: u32) -> Result<(), io::Error> {
    // Retrieve data from src directory about files, dirs, symlinks
    let src_file_sets = file_ops::get_all_files(&src)?;
    let src_files = src_file_sets.files();
    let src_dirs = src_file_sets.dirs();
    let src_symlinks = src_file_sets.symlinks();

    // Retrieve data from dest directory about files, dirs, symlinks
    let dest_file_sets = file_ops::get_all_files(&dest)?;
    let dest_files = dest_file_sets.files();
    let dest_dirs = dest_file_sets.dirs();
    let dest_symlinks = dest_file_sets.symlinks();

    // Determine whether or not to delete
    let delete = !parse::contains_flag(flags, parse::Flag::NoDelete);

    // Delete files and symlinks
    if delete {
        let symlinks_to_delete = dest_symlinks.par_difference(&src_symlinks);
        let files_to_delete = dest_files.par_difference(&src_files);

        file_ops::delete_files(symlinks_to_delete, &dest);
        file_ops::delete_files(files_to_delete, &dest);
    }

    let dirs_to_copy = src_dirs.par_difference(&dest_dirs);
    file_ops::copy_files(dirs_to_copy, &src, &dest);

    let symlinks_to_copy = src_symlinks.par_difference(&dest_symlinks);
    file_ops::copy_files(symlinks_to_copy, &src, &dest);

    let files_to_copy = src_files.par_difference(&dest_files);
    let files_to_compare = src_files.par_intersection(&dest_files);

    file_ops::copy_files(files_to_copy, &src, &dest);
    file_ops::compare_and_copy_files(files_to_compare, &src, &dest, flags);

    // Delete dirs in the correct order
    if delete {
        let dirs_to_delete = dest_dirs.par_difference(&src_dirs);
        let dirs_to_delete: Vec<&file_ops::Dir> = file_ops::sort_files(dirs_to_delete);
        file_ops::delete_files_sequential(dirs_to_delete, &dest);
    }

    Ok(())
}

/// Copies all files, directories, and symlinks in `src` to `dest`
///
/// # Arguments
/// * `src`: Source directory
/// * `dest`: Destination directory
///
/// # Errors
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
/// * `src` is an invalid directory
/// * `dest` is an invalid directory
pub fn copy(src: &str, dest: &str) -> Result<(), io::Error> {
    // Retrieve data from src directory about files, dirs, symlinks
    let src_file_sets = file_ops::get_all_files(&src)?;
    let src_files = src_file_sets.files();
    let src_dirs = src_file_sets.dirs();
    let src_symlinks = src_file_sets.symlinks();

    // Copy everything
    file_ops::copy_files(src_dirs.into_par_iter(), &src, &dest);
    file_ops::copy_files(src_files.into_par_iter(), &src, &dest);
    file_ops::copy_files(src_symlinks.into_par_iter(), &src, &dest);

    Ok(())
}

#[cfg(test)]
mod test_synchronize {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn invalid_src() {
        assert_eq!(synchronize("/?", "src").is_err(), true);
    }

    #[test]
    fn invalid_dest() {
        assert_eq!(synchronize("src", "/?").is_err(), true);
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn dir_1() {
        const TEST_DIR: &str = "test_synchronize_dir1";
        fs::create_dir_all(TEST_DIR).unwrap();

        assert_eq!(synchronize("src", TEST_DIR).is_ok(), true);

        let diff = Command::new("diff")
            .args(&["-r", "src", TEST_DIR])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), true);

        fs::remove_dir_all(TEST_DIR).unwrap();
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn dir_2() {
        const TEST_DIR: &str = "test_synchronize_dir2";
        fs::create_dir_all(TEST_DIR).unwrap();

        assert_eq!(synchronize("target/debug", TEST_DIR).is_ok(), true);

        let diff = Command::new("diff")
            .args(&["-r", "target/debug", TEST_DIR])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), true);

        fs::File::create("target/debug/file.txt").unwrap();
        fs::remove_dir_all("target/debug/build").unwrap();

        let diff = Command::new("diff")
            .args(&["-r", "target/debug", TEST_DIR])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), false);

        assert_eq!(synchronize("target/debug", TEST_DIR).is_ok(), true);

        let diff = Command::new("diff")
            .args(&["-r", "target/debug", TEST_DIR])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), true);

        fs::remove_dir_all(TEST_DIR).unwrap();
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn change_symlink() {
        use std::os::unix::fs::symlink;

        const TEST_SRC: &str = "test_synchronize_change_symlink_src";
        const TEST_DEST: &str = "test_synchronize_change_symlink_dest";
        fs::create_dir_all(TEST_SRC).unwrap();
        fs::create_dir_all(TEST_DEST).unwrap();

        symlink("../Cargo.lock", [TEST_SRC, "file"].join("/")).unwrap();
        symlink("../Cargo.toml", [TEST_DEST, "file"].join("/")).unwrap();

        let diff = Command::new("diff")
            .args(&["-r", TEST_SRC, TEST_DEST])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), false);

        assert_eq!(synchronize(TEST_SRC, TEST_DEST).is_ok(), true);

        let diff = Command::new("diff")
            .args(&["-r", TEST_SRC, TEST_DEST])
            .output()
            .unwrap();

        assert_eq!(diff.status.success(), true);

        fs::remove_dir_all(TEST_DEST).unwrap();
        fs::remove_dir_all(TEST_SRC).unwrap();
    }
}
