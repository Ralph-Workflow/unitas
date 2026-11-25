use unitas_core::normalize_rotation;

const TIME_EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetReference {
    pub guid: u128,
    pub file_id: i64,
}

impl AssetReference {
    pub const NULL: Self = Self {
        guid: 0,
        file_id: 0,
    };

    pub const fn new(guid: u128, file_id: i64) -> Self {
        Self { guid, file_id }
    }

    pub fn from_hex(guid: &str, file_id: i64) -> Option<Self> {
        if guid.len() != 32 || !guid.chars().all(|character| character.is_ascii_hexdigit()) {
            return None;
        }
        let value = u128::from_str_radix(guid, 16).ok()?;
        Some(Self {
            guid: value,
            file_id,
        })
    }

    pub fn guid_hex(&self) -> String {
        format!("{:032x}", self.guid)
    }

    pub const fn is_null(&self) -> bool {
        self.guid == 0 && self.file_id == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimatedValue {
    Float(f32),
    Vector3([f32; 3]),
    Quaternion([f32; 4]),
    Color([f32; 4]),
    ObjectReference(AssetReference),
}

impl AnimatedValue {
    pub fn blend(&self, other: &Self, weight: f32) -> Self {
        let w = if weight.is_finite() {
            weight.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match (self, other) {
            (Self::Float(a), Self::Float(b)) => Self::Float(lerp_scalar(*a, *b, w)),
            (Self::Vector3(a), Self::Vector3(b)) => Self::Vector3([
                lerp_scalar(a[0], b[0], w),
                lerp_scalar(a[1], b[1], w),
                lerp_scalar(a[2], b[2], w),
            ]),
            (Self::Quaternion(a), Self::Quaternion(b)) => {
                Self::Quaternion(nlerp_shortest(*a, *b, w))
            }
            (Self::Color(a), Self::Color(b)) => Self::Color([
                lerp_scalar(a[0], b[0], w),
                lerp_scalar(a[1], b[1], w),
                lerp_scalar(a[2], b[2], w),
                lerp_scalar(a[3], b[3], w),
            ]),
            _ => {
                if w >= 0.5 {
                    *other
                } else {
                    *self
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterpolationMode {
    #[default]
    Hermite,
    Linear,
    Stepped,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub in_tangent: f32,
    pub out_tangent: f32,
    pub mode: InterpolationMode,
}

impl Keyframe {
    pub const fn new(time: f32, value: f32, in_tangent: f32, out_tangent: f32) -> Self {
        Self {
            time,
            value,
            in_tangent,
            out_tangent,
            mode: InterpolationMode::Hermite,
        }
    }

    pub const fn with_mode(
        time: f32,
        value: f32,
        in_tangent: f32,
        out_tangent: f32,
        mode: InterpolationMode,
    ) -> Self {
        Self {
            time,
            value,
            in_tangent,
            out_tangent,
            mode,
        }
    }

    pub const fn linear(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            in_tangent: 0.0,
            out_tangent: 0.0,
            mode: InterpolationMode::Linear,
        }
    }

    pub const fn stepped(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            in_tangent: 0.0,
            out_tangent: 0.0,
            mode: InterpolationMode::Stepped,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatCurve {
    pub keys: Vec<Keyframe>,
}

pub type AnimationCurve = FloatCurve;

impl FloatCurve {
    pub fn new(mut keys: Vec<Keyframe>) -> Self {
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self { keys }
    }

    pub fn clamped_auto(mut keys: Vec<Keyframe>) -> Self {
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        let count = keys.len();
        if count <= 1 {
            for key in &mut keys {
                key.in_tangent = 0.0;
                key.out_tangent = 0.0;
                key.mode = InterpolationMode::Hermite;
            }
            return Self { keys };
        }
        let mut tangents = vec![0.0_f32; count];
        for i in 1..(count - 1) {
            let prev = keys[i - 1];
            let curr = keys[i];
            let next = keys[i + 1];
            let d_prev = curr.value - prev.value;
            let d_next = next.value - curr.value;
            let dt_prev = curr.time - prev.time;
            let dt_next = next.time - curr.time;
            let dt_total = next.time - prev.time;
            if d_prev * d_next <= 0.0
                || dt_prev <= TIME_EPSILON
                || dt_next <= TIME_EPSILON
                || dt_total <= TIME_EPSILON
            {
                tangents[i] = 0.0;
            } else {
                let sec_prev = d_prev / dt_prev;
                let sec_next = d_next / dt_next;
                let slope = (next.value - prev.value) / dt_total;
                let max_mag = (3.0 * sec_prev.abs()).min(3.0 * sec_next.abs());
                tangents[i] = slope.clamp(-max_mag, max_mag);
            }
        }
        for (i, key) in keys.iter_mut().enumerate() {
            key.in_tangent = tangents[i];
            key.out_tangent = tangents[i];
            key.mode = InterpolationMode::Hermite;
        }
        Self { keys }
    }

    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn max_time(&self) -> f32 {
        self.keys.last().map_or(0.0, |k| k.time.max(0.0))
    }

    pub fn evaluate(&self, time: f32) -> f32 {
        let Some(first) = self.keys.first() else {
            return 0.0;
        };
        if !time.is_finite() || self.keys.len() == 1 || time <= first.time {
            return first.value;
        }
        let Some(last) = self.keys.last() else {
            return first.value;
        };
        if time >= last.time {
            return last.value;
        }
        let mut seg = 0usize;
        for idx in 0..(self.keys.len() - 1) {
            if time >= self.keys[idx].time && time <= self.keys[idx + 1].time {
                seg = idx;
                break;
            }
        }
        let k0 = self.keys[seg];
        let k1 = self.keys[seg + 1];
        let dt = k1.time - k0.time;
        if dt <= TIME_EPSILON || (time - k0.time).abs() <= TIME_EPSILON {
            return k0.value;
        }
        if (time - k1.time).abs() <= TIME_EPSILON {
            return k1.value;
        }
        let t = ((time - k0.time) / dt).clamp(0.0, 1.0);
        match k0.mode {
            InterpolationMode::Stepped => k0.value,
            InterpolationMode::Linear => lerp_scalar(k0.value, k1.value, t),
            InterpolationMode::Hermite => {
                let m0 = if k0.out_tangent.is_finite() {
                    k0.out_tangent
                } else {
                    0.0
                };
                let m1 = if k1.in_tangent.is_finite() {
                    k1.in_tangent
                } else {
                    0.0
                };
                hermite_scalar(k0.value, m0, k1.value, m1, t, dt)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectReferenceKey {
    pub time: f32,
    pub reference: AssetReference,
}

impl ObjectReferenceKey {
    pub const fn new(time: f32, reference: AssetReference) -> Self {
        Self { time, reference }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectReferenceCurve {
    pub keys: Vec<ObjectReferenceKey>,
}

impl ObjectReferenceCurve {
    pub fn new(mut keys: Vec<ObjectReferenceKey>) -> Self {
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self { keys }
    }

    pub fn keyframes(&self) -> &[ObjectReferenceKey] {
        &self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn max_time(&self) -> f32 {
        self.keys.last().map_or(0.0, |key| key.time.max(0.0))
    }

    pub fn evaluate(&self, time: f32) -> AssetReference {
        let Some(first) = self.keys.first() else {
            return AssetReference::NULL;
        };
        if !time.is_finite() || time <= first.time {
            return first.reference;
        }
        let mut current = first.reference;
        for key in &self.keys {
            if key.time <= time {
                current = key.reference;
            } else {
                break;
            }
        }
        current
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyCurve {
    Float(FloatCurve),
    Vector3([FloatCurve; 3]),
    Quaternion([FloatCurve; 4]),
    Color([FloatCurve; 4]),
    ObjectReference(ObjectReferenceCurve),
}

impl PropertyCurve {
    pub fn max_time(&self) -> f32 {
        match self {
            Self::Float(c) => c.max_time(),
            Self::Vector3(cs) => cs.iter().map(FloatCurve::max_time).fold(0.0, f32::max),
            Self::Quaternion(cs) | Self::Color(cs) => {
                cs.iter().map(FloatCurve::max_time).fold(0.0, f32::max)
            }
            Self::ObjectReference(c) => c.max_time(),
        }
    }

    pub fn evaluate(&self, time: f32) -> AnimatedValue {
        match self {
            Self::Float(c) => AnimatedValue::Float(c.evaluate(time)),
            Self::Vector3(cs) => AnimatedValue::Vector3([
                cs[0].evaluate(time),
                cs[1].evaluate(time),
                cs[2].evaluate(time),
            ]),
            Self::Quaternion(cs) => {
                AnimatedValue::Quaternion(evaluate_quaternion_channels(cs, time))
            }
            Self::Color(cs) => AnimatedValue::Color([
                cs[0].evaluate(time),
                cs[1].evaluate(time),
                cs[2].evaluate(time),
                cs[3].evaluate(time),
            ]),
            Self::ObjectReference(c) => AnimatedValue::ObjectReference(c.evaluate(time)),
        }
    }
}

fn evaluate_quaternion_channels(channels: &[FloatCurve; 4], time: f32) -> [f32; 4] {
    let mut aligned = channels.clone();
    let count = aligned[0].keys.len();
    if count >= 2 && aligned.iter().all(|c| c.keys.len() == count) {
        for i in 1..count {
            let dot = aligned[0].keys[i - 1].value * aligned[0].keys[i].value
                + aligned[1].keys[i - 1].value * aligned[1].keys[i].value
                + aligned[2].keys[i - 1].value * aligned[2].keys[i].value
                + aligned[3].keys[i - 1].value * aligned[3].keys[i].value;
            if dot < 0.0 {
                for ch in &mut aligned {
                    ch.keys[i].value = -ch.keys[i].value;
                    ch.keys[i].in_tangent = -ch.keys[i].in_tangent;
                    ch.keys[i].out_tangent = -ch.keys[i].out_tangent;
                }
            }
        }
    }
    let raw = [
        aligned[0].evaluate(time),
        aligned[1].evaluate(time),
        aligned[2].evaluate(time),
        aligned[3].evaluate(time),
    ];
    normalize_rotation(raw)
}

fn lerp_scalar(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hermite_scalar(v0: f32, m0: f32, v1: f32, m1: f32, t: f32, dt: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * v0 + h10 * dt * m0 + h01 * v1 + h11 * dt * m1
}

fn nlerp_shortest(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let q0 = normalize_rotation(a);
    let q1 = normalize_rotation(b);
    let dot = q0[0] * q1[0] + q0[1] * q1[1] + q0[2] * q1[2] + q0[3] * q1[3];
    let target = if dot < 0.0 {
        [-q1[0], -q1[1], -q1[2], -q1[3]]
    } else {
        q1
    };
    normalize_rotation([
        lerp_scalar(q0[0], target[0], t),
        lerp_scalar(q0[1], target[1], t),
        lerp_scalar(q0[2], target[2], t),
        lerp_scalar(q0[3], target[3], t),
    ])
}
