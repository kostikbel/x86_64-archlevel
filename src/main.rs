/*-
 * SPDX-License-Identifier: BSD-2-Clause
 *
 * Copyright 2026  Konstantin Belousov <kib@FreeBSD.org>
 * All rights reserved.
 */

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
compile_error!("Only for x86");

use clap::Parser;
#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid, __cpuid_count};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __cpuid_count};

#[derive(Parser, Debug)]
#[command(version,
	  about = "Report the CPU x86_64 architecture level",
	  long_about = None)]
struct AArgs {
    /// Explain why the architecture level reported is the best supported
    #[arg(short, long)]
    explain: bool,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
enum Reg {
    EAX,
    EBX,
    ECX,
    EDX,
}

#[derive(Copy, Clone, Debug)]
struct CPUFeatureDescr {
    name: & 'static str,
    leaf: u32,
    subleaf: u32,
    reg: Reg,
    mask: u32,
}

trait X86CPU {
    fn reg(&self, leaf: u32, subleaf: u32, reg: Reg) -> Option<u32>;
}

impl CPUFeatureDescr {
    fn is_supported<T: X86CPU>(&self, cpu: &T) -> bool {
	let r = cpu.reg(self.leaf, self.subleaf, self.reg);
	match r {
	    None => false,
	    Some(v) => (v & self.mask) == self.mask,
	}
    }
}

trait X8664LevelInterface<'a> {
    fn get_name(&'a self) -> &'a str;
    fn get_parent(&'a self) -> &'a Option::<&'a X8664Level<'a>>;
    fn get_features(&'a self) -> impl Iterator<Item = &'a CPUFeatureDescr>;
    fn is_supported<'b, T: X86CPU>(&'a self, cpu: &'b T) -> bool;
    fn why_not_supported<'b, T: X86CPU>(&'a self, cpu: &'b T) -> String;
}

#[derive(Clone, Debug)]
struct X8664Level<'a> {
    name: String,
    features: Vec<CPUFeatureDescr>,
    parent: Option<&'a X8664Level<'a>>,
}

impl<'a> X8664Level<'a> {
    fn new<'b>(name: &'b str, parent: Option<&'a X8664Level<'a>>) -> X8664Level<'a> {
	X8664Level {
	    name: String::from(name),
	    features: Vec::<CPUFeatureDescr>::new(),
	    parent,
	}
    }
    fn add_feature(&mut self, feat: CPUFeatureDescr) {
	self.features.push(feat);
    }
}

impl<'b> X8664LevelInterface<'b> for X8664Level<'b> {
    fn get_name(&self) -> &str {
	self.name.as_str()
    }
    fn get_features(&'b self) -> impl Iterator<Item = &'b CPUFeatureDescr> {
	self.features.iter()
    }
    fn get_parent(&'b self) -> &'b Option::<&'b X8664Level<'b>> {
	&self.parent
    }
    fn is_supported<'c, T: X86CPU>(&'b self, cpu: &'c T) -> bool {
	let p = match self.parent {
	    None => true,
	    Some(level) => level.is_supported(cpu)
	};
	if p {
	    self.get_features()
		.all(|feat| feat.is_supported(cpu))
	} else {
	    false
	}
    }
    fn why_not_supported<'c, T: X86CPU>(&'b self, cpu: &'c T) -> String {
	let mut res = format!("level {} is not supported because:\n", self.name);
	if let Some(p) = self.parent && !p.is_supported(cpu) {
	    res += format!("parent level {} is not supported;\n", p.name).as_str()
	}
	let resf = self.get_features()
	    .filter(|feat| !feat.is_supported(cpu))
	    .fold(String::from(""), |res, feat| { res + " " + feat.name });
	if !resf.is_empty() {
	    res += format!("the features missing:{}", resf).as_str()
	}
	res
    }
}

struct CurCPU {
    max_level: u32,
    max_ext_level: u32,
}

impl CurCPU {
    fn new() -> CurCPU {
	let regs = __cpuid(0);
	let eregs = __cpuid(0x8000_0000);
	CurCPU {
	    max_level: regs.eax,
	    max_ext_level: eregs.eax,
	}
    }
}

impl X86CPU for CurCPU {
    fn reg(&self, leaf: u32, subleaf: u32, reg: Reg) -> Option<u32> {
	if leaf > self.max_ext_level || (leaf < 0x4000_0000 &&
					 leaf > self.max_level) {
	    None
	} else {
	    let regs = __cpuid_count(leaf, subleaf);
	    match reg {
		Reg::EAX => Some(regs.eax),
		Reg::EBX => Some(regs.ebx),
		Reg::EDX => Some(regs.edx),
		Reg::ECX => Some(regs.ecx),
	    }
	}
    }
}

const CPUID1_LEAF: u32 =              0x0000_0001;

const CPUID_FPU: u32 =                0x0000_0001;
const CPUID_CX8: u32 =                0x0000_0100;
const CPUID_CMOV: u32 =               0x0000_8000;
const CPUID_MMX: u32 =                0x0080_0000;
const CPUID_FXSR: u32 =               0x0100_0000;
const CPUID_SSE2: u32 =               0x0400_0000;

const CPUID2_SSSE3: u32 =             0x0000_0200;
const CPUID2_FMA: u32 =               0x0000_1000;
const CPUID2_CX16: u32 =              0x0000_2000;
const CPUID2_SSE41: u32 =             0x0008_0000;
const CPUID2_SSE42: u32 =             0x0010_0000;
const CPUID2_MOVBE: u32 =             0x0040_0000;
const CPUID2_POPCNT: u32 =            0x0080_0000;
const CPUID2_XSAVE: u32 =             0x0400_0000;
const CPUID2_AVX: u32 =               0x1000_0000;
const CPUID2_F16C: u32 =              0x2000_0000;

const AMDID1_LEAF: u32 =              0x8000_0001;

const AMDID_SYSCALL: u32 =            0x0000_0800;
const AMDID_LM: u32 =                 0x2000_0000;

const AMDID2_LAHF:u32 =               0x0000_0001;
const AMDID2_ABM:u32 =                0x0000_0020;

const CPUID_STDEXT_LEAF: u32 =        0x0000_0007;

const CPUID_STDEXT_BMI1: u32 =        0x0000_0008;
const CPUID_STDEXT_AVX2: u32 =        0x0000_0020;
const CPUID_STDEXT_BMI2: u32 =        0x0000_0100;
const CPUID_STDEXT_AVX512F: u32 =     0x0001_0000;
const CPUID_STDEXT_AVX512DQ: u32 =    0x0002_0000;
const CPUID_STDEXT_AVX512CD: u32 =    0x1000_0000;
const CPUID_STDEXT_AVX512BW: u32 =    0x4000_0000;
const CPUID_STDEXT_AVX512VL: u32 =    0x8000_0000;

fn create_level1<'a>() -> X8664Level<'a> {
    let mut m1 = X8664Level::new("v1", None);
    m1.add_feature(CPUFeatureDescr {
	name: "fpu", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_FPU});
    m1.add_feature(CPUFeatureDescr {
	name: "cmov", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_CMOV});
    m1.add_feature(CPUFeatureDescr {
	name: "fxsr", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_FXSR});
    m1.add_feature(CPUFeatureDescr {
	name: "mmx", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_MMX});
    m1.add_feature(CPUFeatureDescr {
	name: "sse2", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_SSE2});
    m1.add_feature(CPUFeatureDescr {
	name: "cx8", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: CPUID_CX8});
    m1.add_feature(CPUFeatureDescr {
	name: "syscall", leaf: AMDID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: AMDID_SYSCALL});
    m1.add_feature(CPUFeatureDescr {
	name: "lm", leaf: AMDID1_LEAF, subleaf: 0, reg: Reg::EDX,
	mask: AMDID_LM});
    m1
}

fn create_level2<'a>(level1: &'a X8664Level) -> X8664Level<'a> {
    let mut m2 = X8664Level::new("v2", Some(level1));
    m2.add_feature(CPUFeatureDescr {
	name: "cx16", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_CX16});
    m2.add_feature(CPUFeatureDescr {
	name: "lahf", leaf: AMDID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: AMDID2_LAHF});
    m2.add_feature(CPUFeatureDescr {
	name: "popcnt", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_POPCNT});
    m2.add_feature(CPUFeatureDescr {
	name: "sse4.1", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_SSE41});
    m2.add_feature(CPUFeatureDescr {
	name: "sse4.2", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_SSE42});
    m2.add_feature(CPUFeatureDescr {
	name: "ssse3", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_SSSE3});
    m2
}

fn create_level3<'a>(level2: &'a X8664Level) -> X8664Level<'a> {
    let mut m3 = X8664Level::new("v3", Some(level2));
    m3.add_feature(CPUFeatureDescr {
	name: "avx", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_AVX});
    m3.add_feature(CPUFeatureDescr {
	name: "fma", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_FMA});
    m3.add_feature(CPUFeatureDescr {
	name: "movbe", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_MOVBE});
    m3.add_feature(CPUFeatureDescr {
	name: "f16c", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_F16C});
    m3.add_feature(CPUFeatureDescr {
	name: "xsave", leaf: CPUID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: CPUID2_XSAVE});
    m3.add_feature(CPUFeatureDescr {
	name: "abm", leaf: AMDID1_LEAF, subleaf: 0, reg: Reg::ECX,
	mask: AMDID2_ABM});
    m3.add_feature(CPUFeatureDescr {
	name: "bmi1", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_BMI1});
    m3.add_feature(CPUFeatureDescr {
	name: "bmi2", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_BMI2});
    m3.add_feature(CPUFeatureDescr {
	name: "avx2", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX2});
    m3
}

fn create_level4<'a>(level3: &'a X8664Level) -> X8664Level<'a> {
    let mut m4 = X8664Level::new("v4", Some(level3));
    m4.add_feature(CPUFeatureDescr {
	name: "avx512f", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX512F});
    m4.add_feature(CPUFeatureDescr {
	name: "avx512bw", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX512BW});
    m4.add_feature(CPUFeatureDescr {
	name: "avx512cd", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX512CD});
    m4.add_feature(CPUFeatureDescr {
	name: "avx512dq", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX512DQ});
    m4.add_feature(CPUFeatureDescr {
	name: "avx512vl", leaf: CPUID_STDEXT_LEAF, subleaf: 0, reg: Reg::EBX,
	mask: CPUID_STDEXT_AVX512VL});
    m4
}

fn report_max_level<'a, C>(cpu: &'a C, level: &'a X8664Level<'a>,
      args: &AArgs) where C: X86CPU {
    if level.is_supported(cpu) {
	if args.explain {
	    println!("Current CPU x86_64 level is {}", level.get_name());
	} else {
	    println!("{}", level.get_name())
	}
    } else {
	if let Some(l) = level.get_parent() {
	    report_max_level(cpu, l, args)
	}
    }
}

fn report_not_supported<'a, C>(cpu: &'a C, level: &'a X8664Level<'a>)
      where C: X86CPU {
    if !level.is_supported(cpu) {
	if let Some(l) = level.get_parent() {
	    if l.is_supported(cpu) {
		println!("{}", level.why_not_supported(cpu));
	    } else {
		report_not_supported(cpu, l)
	    }
	} else {
	    println!("{}", level.why_not_supported(cpu));
	}
    }
}

fn main() {
    let args = AArgs::parse();

    let cpu = CurCPU::new();
    let level1 = create_level1();
    let level2 = create_level2(&level1);
    let level3 = create_level3(&level2);
    let level4 = create_level4(&level3);

    report_max_level(&cpu, &level4, &args);
    if args.explain {
	report_not_supported(&cpu, &level4)
    }
}
