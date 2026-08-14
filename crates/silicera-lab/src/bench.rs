//! Microbenchmark workloads spanning memory / integer / float / branch / concurrency.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use silicera::topology::format_bytes;
use silicera::topology::TopologyGraph;

/// Workload domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    /// Memory bandwidth / latency sensitive.
    Memory,
    /// Integer ALU.
    Integer,
    /// Floating point.
    Float,
    /// Branch-heavy.
    Branch,
    /// Multi-threaded.
    Concurrency,
}

impl WorkloadKind {
    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            WorkloadKind::Memory => "memory",
            WorkloadKind::Integer => "integer",
            WorkloadKind::Float => "float",
            WorkloadKind::Branch => "branch",
            WorkloadKind::Concurrency => "concurrency",
        }
    }
}

/// Working-set sized relative to a cache level.
#[derive(Debug, Clone, Copy)]
pub enum CacheTarget {
    /// Fit in L1D.
    L1,
    /// Fit in L2.
    L2,
    /// Fit in L3.
    L3,
    /// Exceed L3 (DRAM-resident).
    Dram,
}

impl CacheTarget {
    /// Stable class id used in HNEP `size_classes`.
    pub fn class_id(self) -> &'static str {
        match self {
            CacheTarget::L1 => "L1",
            CacheTarget::L2 => "L2",
            CacheTarget::L3 => "L3",
            CacheTarget::Dram => "DRAM",
        }
    }

    /// Resolve byte size from topology (fractional fit).
    pub fn size_bytes(self, topo: &TopologyGraph) -> u64 {
        let l1 = topo.typical_l1d_bytes().max(32 * 1024);
        let l2 = topo.typical_l2_bytes().max(512 * 1024);
        let l3 = topo
            .packages
            .first()
            .and_then(|p| p.domains.first())
            .and_then(|d| d.l3_bytes())
            .unwrap_or(32 * 1024 * 1024);
        match self {
            CacheTarget::L1 => l1 / 2,
            CacheTarget::L2 => l2 / 2,
            CacheTarget::L3 => l3 / 2,
            CacheTarget::Dram => l3.saturating_mul(4).max(64 * 1024 * 1024),
        }
    }

    /// Upper threshold for this class (L1/L2/L3 boundaries; DRAM uses L3).
    pub fn threshold_bytes(self, topo: &TopologyGraph) -> u64 {
        let l1 = topo.typical_l1d_bytes().max(32 * 1024);
        let l2 = topo.typical_l2_bytes().max(512 * 1024);
        let l3 = topo
            .packages
            .first()
            .and_then(|p| p.domains.first())
            .and_then(|d| d.l3_bytes())
            .unwrap_or(32 * 1024 * 1024);
        match self {
            CacheTarget::L1 => l1,
            CacheTarget::L2 => l2,
            CacheTarget::L3 => l3,
            CacheTarget::Dram => l3,
        }
    }

    /// Label including resolved size.
    pub fn describe(self, topo: &TopologyGraph) -> String {
        format!("{:?} (~{})", self, format_bytes(self.size_bytes(topo)))
    }

    /// All size classes in ascending order.
    pub fn all() -> [CacheTarget; 4] {
        [
            CacheTarget::L1,
            CacheTarget::L2,
            CacheTarget::L3,
            CacheTarget::Dram,
        ]
    }
}

/// Sequential memory touch (baseline-friendly).
pub struct MemoryBench {
    /// Buffer.
    data: Vec<u8>,
    /// Stride.
    stride: usize,
}

impl MemoryBench {
    /// Allocate for target cache level.
    pub fn for_target(topo: &TopologyGraph, target: CacheTarget) -> Self {
        let n = target.size_bytes(topo) as usize;
        Self::with_bytes(n)
    }

    /// Allocate exact byte count.
    pub fn with_bytes(n: usize) -> Self {
        let mut data = vec![0u8; n.max(64)];
        for (i, b) in data.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        Self { data, stride: 64 }
    }

    /// Run once; returns checksum.
    pub fn run(&self) -> u64 {
        let mut sum = 0u64;
        let mut i = 0usize;
        while i < self.data.len() {
            sum = sum.wrapping_add(self.data[i] as u64);
            i += self.stride;
        }
        std::hint::black_box(sum)
    }

    /// Alternate access pattern (candidate).
    pub fn run_prefetch_friendly(&self) -> u64 {
        let mut sum = 0u64;
        for chunk in self.data.chunks(64) {
            for &b in chunk {
                sum = sum.wrapping_add(b as u64);
            }
        }
        std::hint::black_box(sum)
    }
}

/// Flagship memscan / memcpy-style variants for size-class HNEPs.
pub struct MemOpBench {
    src: Vec<u8>,
    dst: Vec<u8>,
}

impl MemOpBench {
    /// Allocate for a cache target (src+dst each sized to the working set).
    pub fn for_target(topo: &TopologyGraph, target: CacheTarget) -> Self {
        let n = target.size_bytes(topo) as usize;
        Self::with_bytes(n)
    }

    /// Exact working-set bytes.
    pub fn with_bytes(n: usize) -> Self {
        let n = n.max(64);
        let mut src = vec![0u8; n];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        let dst = vec![0u8; n];
        Self { src, dst }
    }

    /// Baseline: strided read checksum (scan).
    pub fn run_scan_stride(&self) -> u64 {
        let mut sum = 0u64;
        let mut i = 0usize;
        while i < self.src.len() {
            sum = sum.wrapping_add(self.src[i] as u64);
            i += 64;
        }
        std::hint::black_box(sum)
    }

    /// Dense sequential scan.
    pub fn run_scan_dense(&self) -> u64 {
        let mut sum = 0u64;
        for &b in &self.src {
            sum = sum.wrapping_add(b as u64);
        }
        std::hint::black_box(sum)
    }

    /// Copy via `copy_from_slice` (memcpy stand-in).
    pub fn run_copy(&mut self) -> u64 {
        self.dst.copy_from_slice(&self.src);
        std::hint::black_box(self.dst[0] as u64)
    }

    /// Manual byte loop copy.
    pub fn run_copy_loop(&mut self) -> u64 {
        for i in 0..self.src.len() {
            self.dst[i] = self.src[i];
        }
        std::hint::black_box(self.dst[self.dst.len() - 1] as u64)
    }

    /// Unrolled 8-wide copy (scalar, stronger differentiation vs memcpy).
    pub fn run_copy_unrolled8(&mut self) -> u64 {
        let n = self.src.len();
        let mut i = 0usize;
        while i + 8 <= n {
            self.dst[i] = self.src[i];
            self.dst[i + 1] = self.src[i + 1];
            self.dst[i + 2] = self.src[i + 2];
            self.dst[i + 3] = self.src[i + 3];
            self.dst[i + 4] = self.src[i + 4];
            self.dst[i + 5] = self.src[i + 5];
            self.dst[i + 6] = self.src[i + 6];
            self.dst[i + 7] = self.src[i + 7];
            i += 8;
        }
        while i < n {
            self.dst[i] = self.src[i];
            i += 1;
        }
        std::hint::black_box(self.dst[n - 1] as u64)
    }
}

/// Alignment-aware memory checksum (same bytes, different base alignment).
pub struct AlignmentBench {
    /// Backing store (oversized so we can offset).
    backing: Vec<u8>,
    /// Byte offset into backing (0 = aligned, 1/7/15 = misaligned).
    offset: usize,
    /// Working length.
    len: usize,
}

impl AlignmentBench {
    /// Create with requested alignment offset and working-set length.
    pub fn new(len: usize, offset: usize) -> Self {
        let len = len.max(64);
        let offset = offset % 64;
        let mut backing = vec![0u8; len + offset + 64];
        for (i, b) in backing.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        Self {
            backing,
            offset,
            len,
        }
    }

    /// Slice view used by both variants (identical bytes).
    fn view(&self) -> &[u8] {
        &self.backing[self.offset..self.offset + self.len]
    }

    /// Dense scalar checksum.
    pub fn run_scalar(&self) -> u64 {
        let mut sum = 0u64;
        for &b in self.view() {
            sum = sum.wrapping_add(b as u64);
        }
        std::hint::black_box(sum)
    }

    /// Chunked checksum (64-byte steps) — may interact with alignment differently.
    pub fn run_chunked(&self) -> u64 {
        let mut sum = 0u64;
        for chunk in self.view().chunks(64) {
            for &b in chunk {
                sum = sum.wrapping_add(b as u64);
            }
        }
        std::hint::black_box(sum)
    }

    /// Alignment offset used.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

/// Host-ISA integer reduction: portable scalar vs AVX2 when detected.
///
/// This release's step beyond “algorithm stand-ins”: the fast path uses
/// real `target_feature(enable = "avx2")` code selected at runtime via
/// `is_x86_feature_detected!("avx2")`. It is still **not** a separately
/// compiled `-march=native` binary — see docs for that distinction.
pub struct HostIsaReduce {
    data: Vec<i32>,
}

impl HostIsaReduce {
    /// Create with `n` elements.
    pub fn new(n: usize) -> Self {
        let data = (0..n as i32).map(|i| i.wrapping_mul(3).wrapping_add(1)).collect();
        Self { data }
    }

    /// Portable scalar sum.
    pub fn run_portable(&self) -> i64 {
        let mut s = 0i64;
        for &x in &self.data {
            s = s.wrapping_add(x as i64);
        }
        std::hint::black_box(s)
    }

    /// Host-ISA path: AVX2 when available, else scalar.
    pub fn run_host_isa(&self) -> i64 {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                // SAFETY: feature detected at runtime before calling AVX2 path.
                return std::hint::black_box(unsafe { Self::sum_avx2(&self.data) });
            }
        }
        self.run_portable()
    }

    /// Whether AVX2 was selected on this process.
    pub fn avx2_active() -> bool {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            return is_x86_feature_detected!("avx2");
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            false
        }
    }

    /// Correctness: portable and host-ISA must agree.
    pub fn verify_correctness(&self) -> bool {
        self.run_portable() == self.run_host_isa()
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn sum_avx2(data: &[i32]) -> i64 {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        let mut i = 0usize;
        let n = data.len();
        let mut acc = _mm256_setzero_si256();
        while i + 8 <= n {
            // SAFETY: i..i+8 in-bounds by loop condition; data is aligned enough for loadu.
            let v = unsafe { _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i) };
            acc = _mm256_add_epi32(acc, v);
            i += 8;
        }
        let mut tmp = [0i32; 8];
        // SAFETY: tmp is 32-byte writable stack buffer.
        unsafe {
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, acc);
        }
        let mut s: i64 = tmp.iter().map(|&x| x as i64).sum();
        while i < n {
            s = s.wrapping_add(data[i] as i64);
            i += 1;
        }
        s
    }
}

/// Host-ISA float dot product: portable scalar vs AVX (when detected).
pub struct HostIsaDot {
    a: Vec<f32>,
    b: Vec<f32>,
}

impl HostIsaDot {
    /// Create length-`n` vectors.
    pub fn new(n: usize) -> Self {
        let n = n.max(8);
        let a = (0..n).map(|i| (i as f32) * 0.001 + 1.0).collect();
        let b = (0..n).map(|i| (i as f32) * 0.0007 + 0.5).collect();
        Self { a, b }
    }

    /// Portable scalar dot.
    pub fn run_portable(&self) -> f32 {
        let mut s = 0.0f32;
        for i in 0..self.a.len() {
            s += self.a[i] * self.b[i];
        }
        std::hint::black_box(s)
    }

    /// Host-ISA path.
    pub fn run_host_isa(&self) -> f32 {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx") {
                // SAFETY: AVX detected.
                return std::hint::black_box(unsafe { Self::dot_avx(&self.a, &self.b) });
            }
        }
        self.run_portable()
    }

    /// Relative agreement within tolerance.
    pub fn verify_correctness(&self, rel_tol: f32) -> bool {
        let p = self.run_portable();
        let h = self.run_host_isa();
        let denom = p.abs().max(1.0);
        (p - h).abs() / denom <= rel_tol
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx")]
    unsafe fn dot_avx(a: &[f32], b: &[f32]) -> f32 {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        let n = a.len().min(b.len());
        let mut i = 0usize;
        let mut acc = _mm256_setzero_ps();
        while i + 8 <= n {
            let va = unsafe { _mm256_loadu_ps(a.as_ptr().add(i)) };
            let vb = unsafe { _mm256_loadu_ps(b.as_ptr().add(i)) };
            acc = _mm256_add_ps(acc, _mm256_mul_ps(va, vb));
            i += 8;
        }
        let mut tmp = [0f32; 8];
        unsafe {
            _mm256_storeu_ps(tmp.as_mut_ptr(), acc);
        }
        let mut s: f32 = tmp.iter().sum();
        while i < n {
            s += a[i] * b[i];
            i += 1;
        }
        s
    }
}

#[cfg(test)]
mod host_isa_tests {
    use super::*;

    #[test]
    fn reduce_portable_matches_host_isa() {
        let b = HostIsaReduce::new(1024);
        assert!(b.verify_correctness());
    }

    #[test]
    fn dot_portable_matches_host_isa() {
        let b = HostIsaDot::new(2048);
        assert!(b.verify_correctness(1e-4));
    }
}

/// Integer mix.
pub struct IntegerBench {
    /// Input.
    pub n: u64,
}

impl IntegerBench {
    /// Baseline dependent chain.
    pub fn run_baseline(&self) -> u64 {
        let mut x = self.n;
        for _ in 0..10_000 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        }
        std::hint::black_box(x)
    }

    /// Slightly different mix (candidate — not claimed faster a priori).
    pub fn run_candidate(&self) -> u64 {
        let mut x = self.n;
        for _ in 0..10_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
        }
        std::hint::black_box(x)
    }
}

/// Float mix.
pub struct FloatBench {
    /// Input vector.
    data: Vec<f64>,
}

impl FloatBench {
    /// Create with `n` elements.
    pub fn new(n: usize) -> Self {
        let data = (0..n).map(|i| (i as f64) * 0.5 + 1.0).collect();
        Self { data }
    }

    /// Baseline sum of reciprocals.
    pub fn run_baseline(&self) -> f64 {
        let mut s = 0.0;
        for &x in &self.data {
            s += 1.0 / x;
        }
        std::hint::black_box(s)
    }

    /// Candidate: pairwise summation (often more accurate; timing varies).
    pub fn run_candidate(&self) -> f64 {
        fn pair(a: &[f64]) -> f64 {
            match a.len() {
                0 => 0.0,
                1 => 1.0 / a[0],
                _ => {
                    let mid = a.len() / 2;
                    pair(&a[..mid]) + pair(&a[mid..])
                }
            }
        }
        std::hint::black_box(pair(&self.data))
    }
}

/// Branch-heavy workload.
pub struct BranchBench {
    /// Data.
    data: Vec<u32>,
}

impl BranchBench {
    /// Create.
    pub fn new(n: usize) -> Self {
        let data = (0..n as u32).map(|i| i.wrapping_mul(2654435761)).collect();
        Self { data }
    }

    /// Unpredictable branches.
    pub fn run_baseline(&self) -> u64 {
        let mut c = 0u64;
        for &x in &self.data {
            if x % 3 == 0 {
                c = c.wrapping_add(x as u64);
            } else if x % 5 == 0 {
                c = c.wrapping_add(1);
            } else {
                c = c.wrapping_sub(1);
            }
        }
        std::hint::black_box(c)
    }

    /// Branchless-ish candidate.
    pub fn run_candidate(&self) -> u64 {
        let mut c = 0u64;
        for &x in &self.data {
            let m3 = ((x % 3 == 0) as u64).wrapping_mul(x as u64);
            let m5 = ((x % 5 == 0) as u64) & (((x % 3 != 0) as u64).wrapping_neg());
            let other = (((x % 3 != 0) && (x % 5 != 0)) as u64).wrapping_neg();
            c = c.wrapping_add(m3).wrapping_add(m5 & 1).wrapping_add(other);
        }
        std::hint::black_box(c)
    }
}

/// Shared-counter concurrency microbench.
pub struct ConcurrencyBench {
    /// Iterations per thread.
    pub iters: u64,
    /// Thread count.
    pub threads: usize,
}

impl ConcurrencyBench {
    /// Contended atomic (baseline).
    pub fn run_baseline(&self) -> u64 {
        let counter = Arc::new(AtomicU64::new(0));
        let mut handles = Vec::new();
        for _ in 0..self.threads {
            let c = Arc::clone(&counter);
            let iters = self.iters;
            handles.push(std::thread::spawn(move || {
                for _ in 0..iters {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }
        std::hint::black_box(counter.load(Ordering::Relaxed))
    }

    /// Per-thread locals then reduce (candidate).
    pub fn run_candidate(&self) -> u64 {
        let mut handles = Vec::new();
        for _ in 0..self.threads {
            let iters = self.iters;
            handles.push(std::thread::spawn(move || {
                let mut local = 0u64;
                for _ in 0..iters {
                    local = local.wrapping_add(1);
                }
                local
            }));
        }
        let mut sum = 0u64;
        for h in handles {
            sum = sum.wrapping_add(h.join().unwrap_or(0));
        }
        std::hint::black_box(sum)
    }
}

