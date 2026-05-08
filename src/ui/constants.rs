/*
 * Pro Audio Config - UI Constants Module
 * Version: 2.1
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Common constants for audio settings options
 * Sample rates, bit depths, buffer sizes, and configuration modes
 */

/// Common option definitions to avoid duplication
pub const SAMPLE_RATES: &[(u32, &str)] = &[
    (44100, "44.1 kHz - CD Quality"),
    (48000, "48 kHz - Standard Audio"),
    (96000, "96 kHz - High Resolution"),
    (192000, "192 kHz - Studio Quality"),
    (384000, "384 kHz - Ultra High Resolution"),
];

pub const BIT_DEPTHS: &[(u32, &str)] = &[
    (16, "16 bit - CD Quality"),
    (24, "24 bit - High Resolution"),
    (32, "32 bit - Studio Quality"),
];

pub const BUFFER_SIZES: &[(u32, &str)] = &[
    (128, "128 samples (2.7ms @48kHz)"),
    (256, "256 samples (5.3ms @48kHz)"),
    (512, "512 samples (10.7ms @48kHz)"),
    (1024, "1024 samples (21.3ms @48kHz)"),
    (2048, "2048 samples (42.7ms @48kHz)"),
    (4096, "4096 samples (85.3ms @48kHz)"),
    (8192, "8192 samples (170.7ms @48kHz)"),
];

pub const EXCLUSIVE_BUFFER_SIZES: &[(u32, &str)] = &[
    (64, "64 samples (1.3ms @48kHz) - Ultra Low Latency"),
    (128, "128 samples (2.7ms @48kHz) - Low Latency"),
    (256, "256 samples (5.3ms @48kHz) - Balanced"),
    (512, "512 samples (10.7ms @48kHz) - Stable"),
    (1024, "1024 samples (21.3ms @48kHz) - High Latency"),
];

pub const CONFIG_MODES: &[(&str, &str)] = &[
    ("global", "Global System Settings (All Applications)"),
    ("exclusive", "Exclusive Mode (Single Application)"),
];
