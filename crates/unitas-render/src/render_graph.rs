#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderPassKind {
    Shadow,
    Opaque,
    Transparent,
    Sprites,
    UiOverlay,
    CaptureIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTargetKind {
    ShadowDepth,
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
    pub fn offscreen(has_overlay: bool, shadows: bool) -> Self {
        let mut passes = Vec::new();
        if shadows {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            });
        }
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Opaque,
            target: RenderTargetKind::OffscreenColorDepth,
        });
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Transparent,
            target: RenderTargetKind::OffscreenColorDepth,
        });
        if has_overlay {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::UiOverlay,
                target: RenderTargetKind::OffscreenColorDepth,
            });
        }
        Self { passes }
    }

    pub fn surface(has_overlay: bool, shadows: bool) -> Self {
        let mut passes = Vec::new();
        if shadows {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            });
        }
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Opaque,
            target: RenderTargetKind::SurfaceColorDepth,
        });
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Transparent,
            target: RenderTargetKind::SurfaceColorDepth,
        });
        if has_overlay {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::UiOverlay,
                target: RenderTargetKind::SurfaceColorDepth,
            });
        }
        Self { passes }
    }

    pub fn capture(shadows: bool) -> Self {
        let mut passes = Vec::new();
        if shadows {
            passes.push(RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            });
        }
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Opaque,
            target: RenderTargetKind::CaptureColorIdDepth,
        });
        passes.push(RenderGraphPass {
            kind: RenderPassKind::Transparent,
            target: RenderTargetKind::CaptureColorIdDepth,
        });
        passes.push(RenderGraphPass {
            kind: RenderPassKind::CaptureIds,
            target: RenderTargetKind::CaptureColorIdDepth,
        });
        Self { passes }
    }

    pub fn passes(&self) -> &[RenderGraphPass] {
        &self.passes
    }

    pub fn with_sprites(self, has_sprites: bool) -> Self {
        if !has_sprites || self.has_sprite_pass() {
            return self;
        }
        let target = self
            .passes
            .iter()
            .find(|pass| pass.kind == RenderPassKind::Transparent)
            .map_or(RenderTargetKind::OffscreenColorDepth, |pass| pass.target);
        // The sprite pass draws into the colour target, so it has to run
        // before the pass that reads the target back.
        let index = self
            .passes
            .iter()
            .position(|pass| {
                pass.kind == RenderPassKind::UiOverlay || pass.kind == RenderPassKind::CaptureIds
            })
            .unwrap_or(self.passes.len());
        let mut passes = self.passes;
        passes.insert(
            index,
            RenderGraphPass {
                kind: RenderPassKind::Sprites,
                target,
            },
        );
        Self { passes }
    }

    pub fn has_sprite_pass(&self) -> bool {
        self.passes
            .iter()
            .any(|pass| pass.kind == RenderPassKind::Sprites)
    }

    pub fn pass_order(&self) -> Vec<RenderPassKind> {
        self.passes.iter().map(|pass| pass.kind).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offscreen_pass_order_with_and_without_shadows() {
        let without_shadows = RenderGraph::offscreen(false, false);
        assert_eq!(
            without_shadows.pass_order(),
            vec![RenderPassKind::Opaque, RenderPassKind::Transparent]
        );
        assert_eq!(
            without_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            without_shadows.pass_order()
        );
        assert!(without_shadows
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::OffscreenColorDepth));

        let with_overlay_and_shadows = RenderGraph::offscreen(true, true);
        assert_eq!(
            with_overlay_and_shadows.pass_order(),
            vec![
                RenderPassKind::Shadow,
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::UiOverlay,
            ]
        );
        assert_eq!(
            with_overlay_and_shadows.passes()[0],
            RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            }
        );
        assert!(with_overlay_and_shadows.passes()[1..]
            .iter()
            .all(|pass| pass.target == RenderTargetKind::OffscreenColorDepth));
        assert_eq!(
            with_overlay_and_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            with_overlay_and_shadows.pass_order()
        );
    }

    #[test]
    fn surface_pass_order_with_and_without_shadows() {
        let without_shadows = RenderGraph::surface(true, false);
        assert_eq!(
            without_shadows.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::UiOverlay,
            ]
        );
        assert!(without_shadows
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::SurfaceColorDepth));
        assert_eq!(
            without_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            without_shadows.pass_order()
        );

        let with_shadows = RenderGraph::surface(false, true);
        assert_eq!(
            with_shadows.pass_order(),
            vec![
                RenderPassKind::Shadow,
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
            ]
        );
        assert_eq!(
            with_shadows.passes()[0],
            RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            }
        );
        assert!(with_shadows.passes()[1..]
            .iter()
            .all(|pass| pass.target == RenderTargetKind::SurfaceColorDepth));
        assert_eq!(
            with_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            with_shadows.pass_order()
        );
    }

    #[test]
    fn capture_pass_order_with_and_without_shadows() {
        let without_shadows = RenderGraph::capture(false);
        assert_eq!(
            without_shadows.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::CaptureIds,
            ]
        );
        assert!(without_shadows
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::CaptureColorIdDepth));
        assert_eq!(
            without_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            without_shadows.pass_order()
        );

        let with_shadows = RenderGraph::capture(true);
        assert_eq!(
            with_shadows.pass_order(),
            vec![
                RenderPassKind::Shadow,
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::CaptureIds,
            ]
        );
        assert_eq!(
            with_shadows.passes()[0],
            RenderGraphPass {
                kind: RenderPassKind::Shadow,
                target: RenderTargetKind::ShadowDepth,
            }
        );
        assert!(with_shadows.passes()[1..]
            .iter()
            .all(|pass| pass.target == RenderTargetKind::CaptureColorIdDepth));
        assert_eq!(
            with_shadows
                .passes()
                .iter()
                .map(|pass| pass.kind)
                .collect::<Vec<_>>(),
            with_shadows.pass_order()
        );
    }

    #[test]
    fn with_sprites_inserts_sprite_pass_after_transparent_and_before_overlay() {
        let without = RenderGraph::offscreen(true, false).with_sprites(false);
        assert!(!without.has_sprite_pass());
        assert_eq!(
            without.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::UiOverlay,
            ]
        );

        let with = RenderGraph::offscreen(true, false).with_sprites(true);
        assert!(with.has_sprite_pass());
        assert_eq!(
            with.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::Sprites,
                RenderPassKind::UiOverlay,
            ]
        );
        let idempotent = with.clone().with_sprites(true);
        assert_eq!(idempotent, with);
    }

    #[test]
    fn with_sprites_draws_the_sprite_pass_before_the_capture_readback() {
        let with = RenderGraph::capture(false).with_sprites(true);
        assert!(with.has_sprite_pass());
        assert_eq!(
            with.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::Sprites,
                RenderPassKind::CaptureIds,
            ]
        );
    }
}
