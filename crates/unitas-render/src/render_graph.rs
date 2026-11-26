#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderPassKind {
    Opaque,
    Transparent,
    UiOverlay,
    CaptureIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTargetKind {
    OffscreenColorDepth,
    SurfaceColorDepth,
    CaptureColorIdDepth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderGraphPass {
    pub kind: RenderPassKind,
    pub target: RenderTargetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderGraph {
    passes: Vec<RenderGraphPass>,
}

impl RenderGraph {
    pub fn offscreen(has_overlay: bool) -> Self {
        let mut passes = vec![
            RenderGraphPass {
                kind: RenderPassKind::Opaque,
                target: RenderTargetKind::OffscreenColorDepth,
            },
            RenderGraphPass {
                kind: RenderPassKind::Transparent,
                target: RenderTargetKind::OffscreenColorDepth,
            },
        ];
        if has_overlay {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::UiOverlay,
                target: RenderTargetKind::OffscreenColorDepth,
            });
        }
        Self { passes }
    }

    pub fn surface(has_overlay: bool) -> Self {
        let mut passes = vec![
            RenderGraphPass {
                kind: RenderPassKind::Opaque,
                target: RenderTargetKind::SurfaceColorDepth,
            },
            RenderGraphPass {
                kind: RenderPassKind::Transparent,
                target: RenderTargetKind::SurfaceColorDepth,
            },
        ];
        if has_overlay {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::UiOverlay,
                target: RenderTargetKind::SurfaceColorDepth,
            });
        }
        Self { passes }
    }

    pub fn capture() -> Self {
        Self {
            passes: vec![
                RenderGraphPass {
                    kind: RenderPassKind::Opaque,
                    target: RenderTargetKind::CaptureColorIdDepth,
                },
                RenderGraphPass {
                    kind: RenderPassKind::Transparent,
                    target: RenderTargetKind::CaptureColorIdDepth,
                },
                RenderGraphPass {
                    kind: RenderPassKind::CaptureIds,
                    target: RenderTargetKind::CaptureColorIdDepth,
                },
            ],
        }
    }

    pub fn passes(&self) -> &[RenderGraphPass] {
        &self.passes
    }

    pub fn pass_order(&self) -> Vec<RenderPassKind> {
        self.passes.iter().map(|pass| pass.kind).collect()
    }
}
