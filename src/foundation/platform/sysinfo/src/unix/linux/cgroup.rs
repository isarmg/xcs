// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::get_all_utf8_data;

use std::cmp::min;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Copy)]
struct CGroupLimitsContext {
    mem_total: u64,
    swap_total: u64,
    swap_free: u64,
}

pub(crate) fn limits_for_system() -> Option<crate::CGroupLimits> {
    let v2_base = Path::new("/sys/fs/cgroup");
    let v1_base = Path::new("/sys/fs/cgroup/memory");

    limits_for_base(v2_base, v2_base, v1_base, v1_base)
}

pub(crate) fn limits_for_process(proc_path: &Path) -> Option<crate::CGroupLimits> {
    let cgroup_path = get_cgroup_path(&proc_path.join("cgroup"))?;
    let v2_root = Path::new("/sys/fs/cgroup");
    let v1_root = Path::new("/sys/fs/cgroup/memory");
    let v2_base = v2_root.join(&cgroup_path);
    let v1_base = v1_root.join(&cgroup_path);

    limits_for_base(&v2_base, v2_root, &v1_base, v1_root)
}

fn limits_for_base(
    v2_base: &Path,
    v2_root: &Path,
    v1_base: &Path,
    v1_root: &Path,
) -> Option<crate::CGroupLimits> {
    let context = read_cgroup_limits_context()?;
    v2_limits(v2_base, v2_root, context).or_else(|| v1_limits(v1_base, v1_root, context))
}

fn read_cgroup_limits_context() -> Option<CGroupLimitsContext> {
    let mut mem_total = None;
    let mut swap_total = 0;
    let mut swap_free = 0;

    read_table("/proc/meminfo", ':', |key, value_kib| {
        let value = value_kib.saturating_mul(1_024);
        match key {
            "MemTotal" => mem_total = Some(value),
            "SwapTotal" => swap_total = value,
            "SwapFree" => swap_free = value,
            _ => (),
        }
    });

    Some(CGroupLimitsContext {
        mem_total: mem_total?,
        swap_total,
        swap_free,
    })
}

fn v2_limits(
    base: &Path,
    root: &Path,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    let mem_max = read_v2_memory_max(&base.join("memory.max"));
    let (total_memory, free_memory) = memory_limits(
        base,
        root,
        "memory.max",
        "memory.current",
        context.mem_total,
        mem_max,
        read_v2_memory_max,
    )?;
    let mem_rss = read_table_key(&base.join("memory.stat"), "anon", ' ')?;

    let mut limits = crate::CGroupLimits {
        total_memory,
        free_memory,
        free_swap: context.swap_free,
        rss: mem_rss,
    };

    if let Some(swap_cur) = read_u64(&base.join("memory.swap.current")) {
        limits.free_swap = context.swap_total.saturating_sub(swap_cur);
    }

    Some(limits)
}

fn v1_limits(
    base: &Path,
    root: &Path,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    let mem_max = read_u64(&base.join("memory.limit_in_bytes"))?;
    let (total_memory, free_memory) = memory_limits(
        base,
        root,
        "memory.limit_in_bytes",
        "memory.usage_in_bytes",
        context.mem_total,
        mem_max,
        |path| read_u64(path).unwrap_or(u64::MAX),
    )?;
    let mem_rss = read_table_key(&base.join("memory.stat"), "total_rss", ' ')?;

    Some(crate::CGroupLimits {
        total_memory,
        free_memory,
        free_swap: context.swap_free,
        rss: mem_rss,
    })
}

fn memory_limits<F>(
    base: &Path,
    root: &Path,
    limit_file: &str,
    usage_file: &str,
    mem_total: u64,
    base_limit: u64,
    read_limit: F,
) -> Option<(u64, u64)>
where
    F: Fn(&Path) -> u64,
{
    let mem_cur = read_u64(&base.join(usage_file))?;
    let mut total_memory = mem_total;
    let mut free_memory = mem_total.saturating_sub(mem_cur);

    for (pos, path) in base.ancestors().enumerate() {
        let is_base = pos == 0;
        let mem_max = if is_base {
            base_limit
        } else {
            read_limit(&path.join(limit_file))
        };
        if mem_max <= mem_total {
            let mem_cur = if is_base {
                mem_cur
            } else {
                read_u64(&path.join(usage_file))?
            };
            total_memory = min(total_memory, mem_max);
            free_memory = min(free_memory, mem_max.saturating_sub(mem_cur));
        }
        if path == root {
            return Some((total_memory, free_memory));
        }
    }

    None
}

fn read_v2_memory_max(filename: &Path) -> u64 {
    let content = match get_all_utf8_data(filename, 16_635) {
        Ok(content) => content,
        Err(_) => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            return u64::MAX;
        }
    };
    let content = content.trim();
    if content == "max" {
        return u64::MAX;
    }

    match u64::from_str(content).ok() {
        Some(value) => value,
        None => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            u64::MAX
        }
    }
}

fn read_u64(filename: &Path) -> Option<u64> {
    let result = get_all_utf8_data(filename, 16_635)
        .ok()
        .and_then(|d| u64::from_str(d.trim()).ok());

    if result.is_none() {
        sysinfo_debug!("Failed to read u64 in filename {filename:?}");
    }

    result
}

fn read_table<F>(filename: &str, colsep: char, mut f: F)
where
    F: FnMut(&str, u64),
{
    if let Ok(content) = get_all_utf8_data(filename, 16_635) {
        content
            .split('\n')
            .flat_map(|line| {
                let mut split = line.split(colsep);
                let key = split.next()?;
                let value = split.next()?;
                let value0 = value.trim_start().split(' ').next()?;
                let value0_u64 = u64::from_str(value0).ok()?;
                Some((key, value0_u64))
            })
            .for_each(|(k, v)| f(k, v));
    }
}

fn read_table_key(filename: &Path, target_key: &str, colsep: char) -> Option<u64> {
    if let Ok(content) = get_all_utf8_data(filename, 16_635) {
        return content.split('\n').find_map(|line| {
            let mut split = line.split(colsep);
            let key = split.next()?;
            if key != target_key {
                return None;
            }

            let value = split.next()?;
            let value0 = value.trim_start().split(' ').next()?;
            u64::from_str(value0).ok()
        });
    }

    None
}

fn get_cgroup_path(path: &Path) -> Option<PathBuf> {
    let content = get_all_utf8_data(path, 4096).ok()?;
    parse_cgroup_path(&content)
}

fn parse_cgroup_path(content: &str) -> Option<PathBuf> {
    for line in content.lines() {
        let mut fields = line.splitn(3, ':');
        let hierarchy_id = fields.next()?;
        let controllers = fields.next()?;
        let path = fields.next()?;

        if (hierarchy_id == "0" && controllers.is_empty())
            || controllers
                .split(',')
                .any(|controller| controller == "memory")
        {
            return Some(Path::new(path).strip_prefix("/").ok()?.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/sysinfo/unit/src/unix/linux/cgroup/test_255.rs"));
