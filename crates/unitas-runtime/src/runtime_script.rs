use unitas_core::EntityId;

use crate::runtime::{Callbacks, Runtime, ScriptInstance};

impl Runtime {
    pub(crate) fn refresh_script_view(&mut self) {
        let scene_scripts = match self.active_scene.and_then(|index| self.scenes.get(index)) {
            Some(scene) => scene.scripts.clone(),
            None => Vec::new(),
        };
        let persistent = self.persistent.clone();
        self.script_view = scene_scripts;
        self.script_view.extend(persistent);
        self.pending_scripts = self.scripts_need_init();
    }

    fn scripts_need_init(&self) -> bool {
        if self
            .scenes
            .iter()
            .any(|scene| scene.scripts.iter().any(|script| !script.awake_called))
        {
            return true;
        }
        self.persistent.iter().any(|script| !script.awake_called)
    }

    pub(crate) fn all_script_handles(&self) -> Vec<u64> {
        let mut handles = Vec::new();
        for scene in &self.scenes {
            for script in &scene.scripts {
                handles.push(script.script_handle);
            }
        }
        for script in &self.persistent {
            handles.push(script.script_handle);
        }
        handles
    }

    fn find_script_at(&self, entity: EntityId, ordinal: usize) -> Option<&ScriptInstance> {
        for scene in &self.scenes {
            if let Some(script) = scene.scripts.get(ordinal)
                && script.entity == entity
            {
                return Some(script);
            }
        }
        if let Some(script) = self.persistent.get(ordinal)
            && script.entity == entity
        {
            return Some(script);
        }
        for scene in &self.scenes {
            if let Some(script) = scene.scripts.iter().find(|script| script.entity == entity) {
                return Some(script);
            }
        }
        self.persistent
            .iter()
            .find(|script| script.entity == entity)
    }

    fn find_script_at_mut(
        &mut self,
        entity: EntityId,
        ordinal: usize,
    ) -> Option<&mut ScriptInstance> {
        if let Some(scene_idx) = self.scenes.iter().position(|scene| {
            scene
                .scripts
                .get(ordinal)
                .is_some_and(|script| script.entity == entity)
        }) {
            return self.scenes[scene_idx].scripts.get_mut(ordinal);
        }
        if self
            .persistent
            .get(ordinal)
            .is_some_and(|script| script.entity == entity)
        {
            return self.persistent.get_mut(ordinal);
        }
        for scene in &mut self.scenes {
            if let Some(script) = scene
                .scripts
                .iter_mut()
                .find(|script| script.entity == entity)
            {
                return Some(script);
            }
        }
        self.persistent
            .iter_mut()
            .find(|script| script.entity == entity)
    }

    fn collect_pending_scripts(&self) -> Vec<(EntityId, usize, u64, u64, u64)> {
        let mut pending = Vec::new();
        for scene in &self.scenes {
            for (ordinal, script) in scene.scripts.iter().enumerate() {
                if !script.awake_called {
                    pending.push((
                        script.entity,
                        ordinal,
                        script.guid_hi,
                        script.guid_lo,
                        script.script_handle,
                    ));
                }
            }
        }
        for (ordinal, script) in self.persistent.iter().enumerate() {
            if !script.awake_called {
                pending.push((
                    script.entity,
                    ordinal,
                    script.guid_hi,
                    script.guid_lo,
                    script.script_handle,
                ));
            }
        }
        pending
    }

    pub(crate) fn init_pending_scripts<H>(&mut self, callbacks: Callbacks, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let pending = self.collect_pending_scripts();
        for (entity, ordinal, guid_hi, guid_lo, existing_handle) in pending.iter().copied() {
            if existing_handle != 0 {
                continue;
            }
            let mut created = 0;
            hook(self, &mut || {
                created = (callbacks.create_script)(entity.as_u64(), guid_hi, guid_lo);
            });
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.script_handle = created;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|s| s.script_handle)
            else {
                continue;
            };
            hook(self, &mut || (callbacks.awake)(handle));
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.awake_called = true;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|s| s.script_handle)
            else {
                continue;
            };
            hook(self, &mut || (callbacks.on_enable)(handle));
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.on_enable_called = true;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|s| s.script_handle)
            else {
                continue;
            };
            hook(self, &mut || (callbacks.start)(handle));
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.start_called = true;
            }
        }
    }
}
