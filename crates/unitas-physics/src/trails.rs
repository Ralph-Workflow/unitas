#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailSettings {
    pub time: f32,
    pub width_start: f32,
    pub width_end: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub min_vertex_distance: f32,
}

impl Default for TrailSettings {
    fn default() -> Self {
        Self {
            time: 1.0,
            width_start: 1.0,
            width_end: 1.0,
            start_color: [1.0; 4],
            end_color: [1.0; 4],
            min_vertex_distance: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailPoint {
    pub position: [f32; 3],
    pub age: f32,
    pub width: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trail {
    settings: TrailSettings,
    points: Vec<TrailPoint>,
}

impl Trail {
    pub fn new(settings: TrailSettings) -> Self {
        Self {
            settings: sanitize(settings),
            points: Vec::new(),
        }
    }

    pub fn settings(&self) -> &TrailSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: TrailSettings) {
        self.settings = sanitize(settings);
        self.refresh_fade();
    }

    pub fn points(&self) -> &[TrailPoint] {
        &self.points
    }

    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn update(&mut self, delta_time: f32, position: [f32; 3]) {
        if !delta_time.is_finite() || delta_time <= 0.0 {
            return;
        }
        if !position.iter().all(|value| value.is_finite()) {
            return;
        }
        for point in &mut self.points {
            point.age += delta_time;
        }
        let lifetime = self.settings.time;
        self.points.retain(|point| point.age <= lifetime);
        if self.records(position) {
            self.points.push(TrailPoint {
                position,
                age: 0.0,
                width: self.settings.width_start,
                color: self.settings.start_color,
            });
        }
        self.refresh_fade();
    }

    fn records(&self, position: [f32; 3]) -> bool {
        let minimum = self.settings.min_vertex_distance;
        if minimum <= 0.0 {
            return true;
        }
        match self.points.last() {
            None => true,
            Some(head) => {
                let dx = head.position[0] - position[0];
                let dy = head.position[1] - position[1];
                let dz = head.position[2] - position[2];
                dx * dx + dy * dy + dz * dz >= minimum * minimum
            }
        }
    }

    fn refresh_fade(&mut self) {
        let lifetime = self.settings.time;
        for point in &mut self.points {
            let fade = if lifetime > 0.0 {
                (point.age / lifetime).clamp(0.0, 1.0)
            } else {
                1.0
            };
            point.width = lerp(self.settings.width_start, self.settings.width_end, fade);
            point.color = [
                lerp(
                    self.settings.start_color[0],
                    self.settings.end_color[0],
                    fade,
                ),
                lerp(
                    self.settings.start_color[1],
                    self.settings.end_color[1],
                    fade,
                ),
                lerp(
                    self.settings.start_color[2],
                    self.settings.end_color[2],
                    fade,
                ),
                lerp(
                    self.settings.start_color[3],
                    self.settings.end_color[3],
                    fade,
                ),
            ];
        }
    }
}

fn lerp(start: f32, end: f32, fade: f32) -> f32 {
    start + (end - start) * fade
}

fn sanitize(settings: TrailSettings) -> TrailSettings {
    TrailSettings {
        time: non_negative(settings.time),
        width_start: non_negative(settings.width_start),
        width_end: non_negative(settings.width_end),
        start_color: sanitize_color(settings.start_color),
        end_color: sanitize_color(settings.end_color),
        min_vertex_distance: non_negative(settings.min_vertex_distance),
    }
}

fn sanitize_color(color: [f32; 4]) -> [f32; 4] {
    [
        non_negative(color[0]),
        non_negative(color[1]),
        non_negative(color[2]),
        non_negative(color[3]),
    ]
}

fn non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
