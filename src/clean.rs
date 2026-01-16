use crate::runtime::find_orphaned_runtimes;
use std::fs;

/// 孤立した一時ディレクトリを削除
pub fn clean(dry_run: bool) -> Result<usize, std::io::Error> {
    let orphans = find_orphaned_runtimes();

    if orphans.is_empty() {
        println!("削除対象のディレクトリはありません");
        return Ok(0);
    }

    if dry_run {
        println!("ドライラン - 削除対象:");
        for path in &orphans {
            println!("  {}", path.display());
        }
        println!("合計: {} 個", orphans.len());
    } else {
        for path in &orphans {
            println!("削除: {}", path.display());
            fs::remove_dir_all(path)?;
        }
        println!("{} 個のディレクトリを削除しました", orphans.len());
    }

    Ok(orphans.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_dry_run() {
        // ドライランは常に成功するはず
        let result = clean(true);
        assert!(result.is_ok());
    }
}
