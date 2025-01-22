// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Autogenerated bag thresholds.
//!
//! Generated on 2025-01-04T11:23:50.156667+00:00
//! Arguments
//! Total issuance: 10000000000000000000000
//! Minimum balance: 1000000000000
//! for the vara runtime.

/// Existential weight for this runtime.
#[cfg(any(test, feature = "std"))]
#[allow(unused)]
pub const EXISTENTIAL_WEIGHT: u64 = 1_845_018_450;

/// Constant ratio between bags for this runtime.
#[cfg(any(test, feature = "std"))]
#[allow(unused)]
pub const CONSTANT_RATIO: f64 = 1.1226667214277837;

/// Upper thresholds delimiting the bag list.
pub const THRESHOLDS: [u64; 200] = [
    1_845_018_450,
    2_071_340_814,
    2_325_425_401,
    2_610_677_711,
    2_930_920_987,
    3_290_447_455,
    3_694_075_856,
    4_147_216_030,
    4_655_941_423,
    5_227_070_493,
    5_868_258_093,
    6_588_098_074,
    7_396_238_465,
    8_303_510_788,
    9_322_075_233,
    10_465_583_639,
    11_749_362_472,
    13_190_618_245,
    14_808_668_139,
    16_625_198_908,
    18_664_557_551,
    20_954_077_633,
    23_524_445_637,
    26_410_112_257,
    29_649_754_140,
    33_286_792_271,
    37_369_973_946,
    41_954_026_130,
    47_100_388_966,
    52_878_039_258,
    59_364_414_969,
    66_646_453_123,
    74_821_755_022,
    83_999_894_402,
    94_303_886_049,
    105_871_834_569,
    118_858_785_407,
    133_438_802_926,
    149_807_303_392,
    168_183_674_145,
    188_814_214_050,
    211_975_434_646,
    237_977_766_237,
    267_169_718_594,
    299_942_552_039,
    336_735_521_514,
    378_041_763_926,
    424_414_907_670,
    476_476_492_919,
    534_924_302_143,
    600_541_712_499,
    674_208_195_452,
    756_911_104_348,
    849_758_907_931,
    953_996_047_171,
    1_071_019_614_533,
    1_202_398_079_233,
    1_349_892_309_464,
    1_515_479_173_347,
    1_701_378_034_934,
    1_910_080_500_389,
    2_144_383_813_035,
    2_407_428_344_863,
    2_702_739_687_000,
    3_034_275_903_277,
    3_406_480_580_239,
    3_824_342_384_624,
    4_293_461_926_563,
    4_820_126_824_669,
    5_411_395_979_117,
    6_075_194_182_223,
    6_820_418_334_593,
    7_657_056_690_463,
    8_596_322_730_469,
    9_650_805_456_151,
    10_834_638_120_594,
    12_163_687_656_704,
    13_655_767_342_023,
    15_330_875_550_450,
    17_211_463_790_841,
    19_322_737_625_036,
    21_692_994_498_508,
    24_354_003_011_591,
    27_341_428_714_665,
    30_695_312_134_244,
    34_460_605_436_954,
    38_687_774_924_322,
    43_433_477_433_625,
    48_761_319_710_615,
    54_742_710_932_008,
    61_457_819_804_106,
    68_996_649_065_575,
    77_460_241_795_952,
    86_962_035_698_065,
    97_629_383_505_833,
    109_605_259_895_509,
    123_050_177_778_131,
    138_144_339_657_280,
    155_090_052_886_845,
    174_114_441_200_536,
    195_472_488_855_836,
    219_450_458_193_110,
    246_369_726_415_484,
    276_591_093_013_931,
    310_519_615_570_077,
    348_610_038_751_074,
    391_372_889_261_481,
    439_381_318_442_906,
    493_278_784_232_914,
    553_787_675_444_649,
    621_718_993_958_558,
    697_983_224_596_834,
    783_602_538_369_720,
    879_724_492_654_023,
    987_637_411_927_612,
    1_108_787_655_208_194,
    1_244_799_001_632_183,
    1_397_494_413_998_981,
    1_568_920_471_977_878,
    1_761_374_802_456_335,
    1_977_436_874_679_164,
    2_220_002_572_926_460,
    2_492_323_010_108_593,
    2_798_048_102_497_639,
    3_141_275_489_628_256,
    3_526_605_455_042_410,
    3_959_202_583_981_800,
    4_444_864_984_427_257,
    4_990_101_999_256_105,
    5_602_221_451_095_080,
    6_289_427_589_213_314,
    7_060_931_051_239_561,
    7_927_072_313_522_752,
    8_899_460_284_743_544,
    9_991_127_900_349_804,
    11_216_706_803_251_370,
    12_592_623_452_022_932,
    14_137_319_285_057_204,
    15_871_497_891_532_950,
    17_818_402_502_035_278,
    20_004_127_518_040_564,
    22_457_968_255_701_908,
    25_212_813_591_558_104,
    28_305_586_772_804_400,
    31_777_740_300_313_956,
    35_675_811_517_337_024,
    40_052_046_350_444_320,
    44_965_099_562_726_952,
    50_480_820_904_760_536,
    56_673_137_700_130_632,
    63_625_045_694_830_984,
    71_429_721_450_908_824,
    80_191_771_193_791_648,
    90_028_632_851_621_056,
    101_072_150_078_155_072,
    113_470_339_355_899_264,
    127_389_373_863_985_440,
    143_015_810_700_618_736,
    160_559_091_311_600_192,
    180_254_348_638_218_336,
    202_365_558_608_769_248,
    227_189_078_213_208_992,
    255_057_617_581_823_648,
    286_344_699_305_767_392,
    321_469_664_767_830_464,
    360_903_294_583_388_928,
    405_174_118_482_418_816,
    454_875_499_204_049_472,
    510_673_585_349_236_672,
    573_316_239_783_798_976,
    643_643_063_259_382_656,
    722_596_647_599_146_752,
    811_235_209_274_841_600,
    910_746_772_703_368_448,
    1_022_465_093_361_825_536,
    1_147_887_534_138_873_472,
    1_288_695_134_519_512_064,
    1_446_775_141_590_957_312,
    1_624_246_304_853_137_664,
    1_823_487_273_860_664_576,
    2_047_168_479_310_439_424,
    2_298_287_924_877_752_576,
    2_580_211_369_519_570_944,
    2_896_717_438_809_228_288,
    3_252_048_269_930_642_944,
    3_650_966_369_127_930_880,
    4_098_818_443_671_953_408,
    4_601_607_063_884_922_880,
    5_166_071_115_710_616_576,
    5_799_776_122_137_610_240,
    6_511_215_644_055_375_872,
    7_309_925_119_620_943_872,
    8_206_609_667_927_444_480,
    9_213_287_569_929_656_320,
    10_343_451_349_704_280_064,
    11_612_248_615_020_288_000,
    13_036_685_081_029_148_672,
    14_635_852_498_205_495_296,
    16_431_184_539_461_001_216,
    18_446_744_073_709_551_615,
];
