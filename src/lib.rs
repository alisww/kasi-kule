#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::f32::consts::PI;
pub mod consts;
pub(crate) mod utils;
use consts::{UCS, VC};
use utils::*;

#[derive(Default, Debug, Copy, Clone)]
pub struct sRGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<[u8; 3]> for sRGB {
    fn from(rgb: [u8; 3]) -> sRGB {
        sRGB {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
}

impl From<(u8, u8, u8)> for sRGB {
    fn from(rgb: (u8, u8, u8)) -> sRGB {
        sRGB {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct LinearRGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl From<&sRGB> for LinearRGB {
    fn from(srgb: &sRGB) -> LinearRGB {
        LinearRGB {
            r: linearize_channel(srgb.r),
            g: linearize_channel(srgb.g),
            b: linearize_channel(srgb.b),
        }
    }
}

impl<T: Into<sRGB>> From<T> for LinearRGB {
    fn from(rgb: T) -> LinearRGB {
        LinearRGB::from(&rgb.into())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct XYZ {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<&LinearRGB> for XYZ {
    fn from(rgb: &LinearRGB) -> XYZ {
        XYZ {
            x: ((rgb.r * 0.4124) + (rgb.g * 0.3576) + (rgb.b * 0.1805)) * 100.0,
            y: ((rgb.r * 0.2126) + (rgb.g * 0.7152) + (rgb.b * 0.0722)) * 100.0,
            z: ((rgb.r * 0.0193) + (rgb.g * 0.1192) + (rgb.b * 0.9505)) * 100.0,
        }
    }
}

impl<T: Into<sRGB>> From<T> for XYZ {
    fn from(rgb: T) -> XYZ {
        XYZ::from(&LinearRGB::from(&rgb.into()))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LMS {
    pub l: f32,
    pub m: f32,
    pub s: f32,
}

impl From<&XYZ> for LMS {
    fn from(xyz: &XYZ) -> LMS {
        LMS {
            l: (0.7328 * xyz.x) + (0.4296 * xyz.y) - (0.1624 * xyz.z),
            m: (-0.7036 * xyz.x) + (1.6975 * xyz.y) + (0.0061 * xyz.z),
            s: (0.0030 * xyz.x) + (0.0136 * xyz.y) + (0.9834 * xyz.z),
        }
    }
}

impl<T: Into<sRGB>> From<T> for LMS {
    fn from(rgb: T) -> LMS {
        LMS::from(&XYZ::from(&LinearRGB::from(&rgb.into())))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HPE {
    pub lh: f32,
    pub mh: f32,
    pub sh: f32,
}

impl From<&LMS> for HPE {
    fn from(lms: &LMS) -> HPE {
        HPE {
            lh: (0.7409792 * lms.l) + (0.2180250 * lms.m) + (0.0410058 * lms.s),
            mh: (0.2853532 * lms.l) + (0.6242014 * lms.m) + (0.0904454 * lms.s),
            sh: (-0.0096280 * lms.l) - (0.0056980 * lms.m) + (1.0153260 * lms.s),
        }
    }
}

impl<T: Into<sRGB>> From<T> for HPE {
    fn from(rgb: T) -> HPE {
        HPE::from(&LMS::from(&XYZ::from(&LinearRGB::from(&rgb.into()))))
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct JCh {
    pub J: f32,
    pub C: f32,
    pub H: f32,
    pub h: f32,
    pub Q: f32,
    pub M: f32,
    pub s: f32,
}

impl From<&LMS> for JCh {
    fn from(lms: &LMS) -> JCh {
        let (lc, mc, sc) = (
            c_transform(lms.l, consts::D65_LMS.l),
            c_transform(lms.m, consts::D65_LMS.m),
            c_transform(lms.s, consts::D65_LMS.s),
        );

        let hpe_transforms = HPE::from(&LMS {
            l: lc,
            m: mc,
            s: sc,
        });

        let (lpa, mpa, spa) = (
            nonlinear_adaptation(hpe_transforms.lh, *VC::fl),
            nonlinear_adaptation(hpe_transforms.mh, *VC::fl),
            nonlinear_adaptation(hpe_transforms.sh, *VC::fl),
        );

        let ca = lpa - ((12.0 * mpa) / 11.0) + (spa / 11.0);
        let cb = (1.0 / 9.0) * (lpa + mpa - 2.0 * spa);

        let mut result_color = JCh::default();

        result_color.h = (180.0 / PI) * cb.atan2(ca);
        if result_color.h < 0.0 {
            result_color.h += 360.0;
        }

        let H = match result_color.h {
            h if h < 20.14 => {
                let temp = ((h + 122.47) / 1.2) + ((20.14 - h) / 0.8);
                300.0 + (100.0 * ((h + 122.47) / 1.2)) / temp
            }
            h if h < 90.0 => {
                let temp = ((h - 20.14) / 0.8) + ((90.0 - h) / 0.7);
                (100.0 * ((h - 20.14) / 0.8)) / temp
            }

            h if h < 164.25 => {
                let temp = ((h - 90.0) / 0.7) + ((164.25 - h) / 1.0);
                100.0 + ((100.0 * ((h - 90.0) / 0.7)) / temp)
            }
            h if h < 237.53 => {
                let temp = ((h - 164.25) / 1.0) + ((237.53 - h) / 1.2);
                200.0 + ((100.0 * ((h - 164.25) / 1.0)) / temp)
            }
            h => {
                let temp = ((h - 237.53) / 1.2) + ((360.0 - h + 20.14) / 0.8);
                300.0 + ((100.0 * ((h - 237.53) / 1.2)) / temp)
            }
        };

        result_color.H = H;

        let a = (2.0 * lpa + mpa + 0.05 * spa - 0.305) * *VC::nbb;
        result_color.J = 100.0 * (a / *VC::achromatic_response_to_white).powf(VC::c * *VC::z);

        let et = 0.25 * (((result_color.h * PI) / 180.0 + 2.0).cos() + 3.8);
        let t = (50000.0 / 13.0) * VC::nc * *VC::ncb * et * (ca.powi(2) + cb.powi(2)).sqrt()
            / (lpa + mpa + (21.0 / 20.0) * spa);

        result_color.C = t.powf(0.9f32)
            * (result_color.J / 100.0).sqrt()
            * (1.64 - 0.29f32.powf(VC::n)).powf(0.73f32);

        result_color.Q = (4.0 / VC::c)
            * (result_color.J / 100.0).sqrt()
            * (*VC::achromatic_response_to_white + 4.0f32)
            * VC::fl.powf(0.25f32);

        result_color.M = result_color.C * VC::fl.powf(0.25f32);

        result_color.s = 100.0 * (result_color.M / result_color.Q).sqrt();

        result_color
    }
}

impl<T: Into<sRGB>> From<T> for JCh {
    fn from(rgb: T) -> JCh {
        JCh::from(&LMS::from(&XYZ::from(&LinearRGB::from(&rgb.into()))))
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Jab {
    pub J: f32,
    pub a: f32,
    pub b: f32,
}

impl From<&JCh> for Jab {
    fn from(cam02: &JCh) -> Jab {
        let j_prime = ((1.0 + 100.0 * UCS::c1) * cam02.J) / (1.0 + UCS::c1 * cam02.J) / UCS::k_l;

        let m_prime = (1.0 / UCS::c2) * (1.0 + UCS::c2 * cam02.M).ln();

        Jab {
            J: j_prime,
            a: m_prime * ((PI / 180.0) * cam02.h).cos(),
            b: m_prime * ((PI / 180.0) * cam02.h).sin(),
        }
    }
}

impl<T: Into<sRGB>> From<T> for Jab {
    fn from(rgb: T) -> Jab {
        Jab::from(&JCh::from(&LMS::from(&XYZ::from(&LinearRGB::from(
            &rgb.into(),
        )))))
    }
}

impl From<[f32; 3]> for Jab {
    fn from(jab: [f32; 3]) -> Jab {
        Jab {
            J: jab[0],
            a: jab[1],
            b: jab[2],
        }
    }
}

impl From<(f32, f32, f32)> for Jab {
    fn from(jab: (f32, f32, f32)) -> Jab {
        Jab {
            J: jab.0,
            a: jab.1,
            b: jab.2,
        }
    }
}

impl Jab {
    pub fn squared_difference(&self, other: &Jab) -> f32 {
        let diff_j = (self.J - other.J).abs();
        let diff_a = (self.a - other.a).abs();
        let diff_b = (self.b - other.b).abs();

        (diff_j / UCS::k_l).powi(2) + diff_a.powi(2) + diff_b.powi(2)
    }
}

#[cfg(test)]
mod tests {
    use crate::{JCh, Jab};

    macro_rules! float_eq {
        ($lhs:expr, $rhs:expr) => {
            assert_eq!(format!("{:.2}", $lhs), $rhs)
        };
    }

    // based on https://github.com/connorgr/d3-cam02/blob/master/test/cam02-test.js,
    #[test]
    fn jch_channels() {
        float_eq!(JCh::from([0, 0, 0]).J, "0.00");
        float_eq!(JCh::from([50, 50, 50]).J, "14.92");
        float_eq!(JCh::from([100, 100, 100]).J, "32.16");
        float_eq!(JCh::from([150, 150, 150]).J, "52.09");
        float_eq!(JCh::from([200, 200, 200]).J, "74.02");
        float_eq!(JCh::from([250, 250, 250]).J, "97.57");
        float_eq!(JCh::from([255, 255, 255]).J, "100.00");

        let red = JCh::from([255, 0, 0]);
        float_eq!(red.J, "46.93");
        float_eq!(red.C, "111.30");
        float_eq!(red.h, "32.15");
    }

    #[test]
    fn jab_channels() {
        float_eq!(Jab::from([0, 0, 0]).J, "0.00");
        float_eq!(Jab::from([50, 50, 50]).J, "22.96");
        float_eq!(Jab::from([150, 150, 150]).J, "64.89");
        let white = Jab::from([255, 255, 255]);
        float_eq!(white.J, "100.00");
        float_eq!(white.a, "-1.91");
        float_eq!(white.b, "-1.15");
        let red = Jab::from([255, 0, 0]);
        float_eq!(red.J, "60.05");
        float_eq!(red.a, "38.69");
        float_eq!(red.b, "24.32");
        let blue = Jab::from([0, 0, 255]);
        float_eq!(blue.J, "31.22");
        float_eq!(blue.a, "-8.38");
        float_eq!(blue.b, "-39.16");
    }
}