// This file is part of Gear.

// Copyright (C) 2021-2025 Gear Technologies Inc.
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

use crate::*;

/// All migrations that will run on the next runtime upgrade.
pub type Migrations = (
    BagsListMigrate<Runtime>,
    pallet_gear_bank::migrations::MigrateToV1<Runtime>,
);

pub struct BagsListMigrate<T: pallet_bags_list::Config<pallet_bags_list::Instance1>>(
    core::marker::PhantomData<T>,
);

impl<T: pallet_bags_list::Config<pallet_bags_list::Instance1>>
    frame_support::traits::OnRuntimeUpgrade for BagsListMigrate<T>
{
    fn on_runtime_upgrade() -> Weight {
        use sp_core::Get;

        let old_thresholds = vec![
            10_000_000_000_000,
            10_751_944_990_983,
            11_560_432_108_912,
            12_429_713_010_701,
            13_364_359_054_476,
            14_369_285_339_347,
            15_449_776_552_839,
            16_611_514_761_910,
            17_860_609_293_695,
            19_203_628_863_124,
            20_647_636_116_355,
            22_200_224_771_687,
            23_869_559_553_263,
            25_664_419_127_567,
            27_594_242_268_512,
            29_669_177_493_889,
            31_900_136_434_199,
            34_298_851_214_535,
            36_877_936_151_258,
            39_650_954_087_929,
            42_632_487_719_339,
            45_838_216_278_707,
            49_284_997_991_342,
            52_990_958_728_360,
            56_975_587_326_676,
            61_259_838_076_535,
            65_866_240_915_541,
            70_819_019_908_670,
            76_144_220_637_332,
            81_869_847_167_384,
            88_026_009_316_387,
            94_645_080_994_551,
            101_761_870_452_051,
            109_413_803_327_995,
            117_641_119_463_679,
            126_487_084_515_109,
            135_998_217_477_622,
            146_224_535_319_108,
            157_219_816_008_304,
            169_041_881_321_369,
            181_752_900_913_957,
            195_419_719_257_838,
            210_114_207_161_354,
            225_913_639_722_280,
            242_901_102_700_662,
            261_165_929_448_653,
            280_804_170_695_076,
            301_919_099_655_195,
            324_621_755_121_965,
            349_031_525_394_759,
            375_276_776_116_320,
            403_495_525_319_597,
            433_836_169_234_393,
            466_458_262_670_681,
            501_533_358_082_447,
            539_245_907_724_525,
            579_794_233_646_657,
            623_391_570_625_777,
            670_267_187_521_060,
            720_667_592_948_706,
            774_857_831_616_833,
            833_122_878_137_625,
            895_769_135_646_485,
            963_126_047_109_104,
            1_035_547_827_789_960,
            1_113_415_327_992_917,
            1_197_138_035_869_669,
            1_287_156_230_828_364,
            1_383_943_298_866_707,
            1_488_008_222_005_383,
            1_599_898_254_913_170,
            1_720_201_802_799_546,
            1_849_551_515_708_982,
            1_988_627_615_489_147,
            2_138_161_472_928_821,
            2_298_939_453_876_902,
            2_471_807_054_568_403,
            2_657_673_347_904_220,
            2_857_515_764_066_674,
            3_072_385_230_611_050,
            3_303_411_699_063_740,
            3_551_810_087_090_170,
            3_818_886_667_481_063,
            4_106_045_937_555_325,
            4_414_798_005_104_243,
            4_746_766_529_718_054,
            5_103_697_261_256_589,
            5_487_467_220_365_940,
            5_900_094_569_319_478,
            6_343_749_225_091_820,
            6_820_764_270_477_575,
            7_333_648_223_263_445,
            7_885_098_227_974_584,
            8_478_014_239_567_711,
            9_115_514_273_659_920,
            9_800_950_803_490_818,
            10_537_928_389_846_000,
            11_330_322_636_653_804,
            12_182_300_571_938_664,
            13_098_342_561_310_038,
            14_083_265_869_225_166,
            15_142_249_991_929_174,
            16_280_863_895_292_920,
            17_505_095_300_786_418,
            18_821_382_173_596_360,
            20_236_646_578_476_860,
            21_758_331_081_373_944,
            23_394_437_888_251_960,
            25_153_570_926_944_424,
            27_044_981_093_328_568,
            29_078_614_899_763_324,
            31_265_166_781_622_228,
            33_616_135_336_989_872,
            36_143_883_795_274_144,
            38_861_705_032_725_504,
            41_783_891_476_765_632,
            44_925_810_266_737_080,
            48_303_984_066_327_840,
            51_936_177_952_645_696,
            55_841_492_838_872_984,
            60_040_465_921_791_112,
            64_555_178_682_406_344,
            69_409_373_007_628_560,
            74_628_576_043_661_488,
            80_240_234_437_681_024,
            86_273_858_673_749_408,
            92_761_178_261_996_064,
            99_736_308_597_171_264,
            107_235_930_364_045_040,
            115_299_482_433_105_296,
            123_969_369_260_951_280,
            133_291_183_886_055_744,
            143_313_947_692_581_776,
            154_090_368_203_119_712,
            165_677_116_256_019_744,
            178_135_124_024_935_328,
            191_529_905_447_796_768,
            205_931_900_750_280_960,
            221_416_846_875_550_944,
            238_066_175_768_274_144,
            255_967_442_607_407_840,
            275_214_786_239_734_496,
            295_909_424_235_466_048,
            318_160_185_169_306_432,
            342_084_080_926_122_112,
            367_806_922_040_850_496,
            395_463_979_308_585_024,
            425_200_695_144_098_752,
            457_173_448_431_691_200,
            491_550_376_887_536_448,
            528_512_261_259_155_712,
            568_253_476_011_826_688,
            610_983_011_501_381_120,
            656_925_573_008_773_888,
            706_322_762_416_006_144,
            759_434_348_737_577_088,
            816_539_634_188_912_512,
            877_938_922_975_624_320,
            943_955_100_527_642_240,
            1_014_935_331_483_065_984,
            1_091_252_885_351_061_632,
            1_173_309_099_454_565_632,
            1_261_535_489_475_482_368,
            1_356_396_018_701_269_504,
            1_458_389_537_906_385_664,
            1_568_052_408_699_399_168,
            1_685_961_324_131_370_496,
            1_812_736_341_398_468_096,
            1_949_044_142_587_138_304,
            2_095_601_540_609_376_000,
            2_253_179_248_765_040_128,
            2_422_605_933_754_521_088,
            2_604_772_573_455_664_640,
            2_800_637_142_381_549_056,
            3_011_229_649_458_912_256,
            3_237_657_554_619_804_160,
            3_481_111_592_691_138_048,
            3_742_872_035_208_694_272,
            4_024_315_423_085_096_960,
            4_326_921_805_537_383_424,
            4_652_282_523_342_105_600,
            5_002_108_577_348_404_224,
            5_378_239_626_257_227_776,
            5_782_653_660_984_053_760,
            6_217_477_406_480_468_992,
            6_684_997_505_715_516_416,
            7_187_672_544_630_912_000,
            7_728_145_981_306_749_952,
            8_309_260_047_329_342_464,
            8_934_070_694_465_457_152,
            9_605_863_665_244_231_680,
            10_328_171_771_958_448_128,
            11_104_793_474_951_653_376,
            11_939_812_857_890_269_184,
            12_837_621_105_066_293_248,
            13_802_939_593_675_003_904,
            14_840_844_722_504_937_472,
            15_956_794_607_608_752_128,
            17_156_657_785_341_708_288,
        ];

        let affected_accounts =
            pallet_bags_list::List::<Runtime, pallet_bags_list::Instance1>::migrate(
                &old_thresholds,
            );

        T::DbWeight::get().reads_writes(affected_accounts.into(), affected_accounts.into())
    }
}
