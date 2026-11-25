fn main() {
    let device = std::env::var("CARGO_FEATURE_DEVICE").is_ok();
    export(device);
}

fn export(device: bool) {
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_active_scene_index17h4a76041104c0b0d8E,_unitas_runtime_active_scene_index"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_active_scene_index17h1b47ec3c6441dbfaE,_unitas_runtime_active_scene_index"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_active_scene_index");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_animator_attach17hfb2b5ef8334dc20eE,_unitas_runtime_animator_attach"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_animator_attach17he32b6ad23f248755E,_unitas_runtime_animator_attach"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_attach");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_animator_controller_create17h67048097e94bd84bE,_unitas_runtime_animator_controller_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_animator_controller_create17h609e0dc68cc06c56E,_unitas_runtime_animator_controller_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_controller_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_animator_controller_destroy17h80aab9e7f8531a0eE,_unitas_runtime_animator_controller_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_animator_controller_destroy17h7315b43018529959E,_unitas_runtime_animator_controller_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_controller_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_get_bool17hd0848bc1cfd9375dE,_unitas_runtime_animator_get_bool"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_get_bool17hd6d9abfb5743791cE,_unitas_runtime_animator_get_bool"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_bool");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_float17ha9bb5bef7a55c6ffE,_unitas_runtime_animator_get_float"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_float17hf3729008c5d13635E,_unitas_runtime_animator_get_float"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_float");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_get_int17h6e00683955adc3efE,_unitas_runtime_animator_get_int"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_get_int17h7d938536cf58bbfeE,_unitas_runtime_animator_get_int"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_int");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_speed17hecd657017181fd3eE,_unitas_runtime_animator_get_speed"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_speed17h7f25011afe3724feE,_unitas_runtime_animator_get_speed"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_speed");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_animator_reset_trigger17h14f4f1dbed989be8E,_unitas_runtime_animator_reset_trigger"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_animator_reset_trigger17h8e13853849c0a5fdE,_unitas_runtime_animator_reset_trigger"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_reset_trigger"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_set_bool17ha19f48e38a87970cE,_unitas_runtime_animator_set_bool"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_set_bool17heec47c8cc987baecE,_unitas_runtime_animator_set_bool"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_bool");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_animator_set_controller17h1aa6561d8eecc13dE,_unitas_runtime_animator_set_controller"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_animator_set_controller17h5c51b0c1184c5d94E,_unitas_runtime_animator_set_controller"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_controller"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_float17hdc9d1dc430f45a16E,_unitas_runtime_animator_set_float"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_float17h21470ff942e62575E,_unitas_runtime_animator_set_float"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_float");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_set_int17h996a1d671cd1a84cE,_unitas_runtime_animator_set_int"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_set_int17h484e3a78ae4ba3b8E,_unitas_runtime_animator_set_int"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_int");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_speed17h4b666de181062b22E,_unitas_runtime_animator_set_speed"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_speed17hcfd4b9730f6b7d1aE,_unitas_runtime_animator_set_speed"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_speed");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_animator_set_trigger17hc04e50c83015eef8E,_unitas_runtime_animator_set_trigger"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_animator_set_trigger17h51944958b6a914a5E,_unitas_runtime_animator_set_trigger"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_trigger"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_info17h10812711f1dad7ebE,_unitas_runtime_animator_state_info"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_info17h0057d75fa21d2214E,_unitas_runtime_animator_state_info"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_state_info"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_name17h93d50f5c4eaf8c28E,_unitas_runtime_animator_state_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_name17hbcb1183df5049be2E,_unitas_runtime_animator_state_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_state_name"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_clear_listener17h3d12e17ef48e0ee4E,_unitas_runtime_audio_clear_listener"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_clear_listener17h12fab190e9236df5E,_unitas_runtime_audio_clear_listener"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_clear_listener"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_clip_info17h1f544faf63ee710bE,_unitas_runtime_audio_clip_info"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_clip_info17hdf9fab643bfbdf11E,_unitas_runtime_audio_clip_info"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_clip_info");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_audio_create17h72b3d4899d065205E,_unitas_runtime_audio_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_audio_create17h53c8ce38cb78a443E,_unitas_runtime_audio_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_destroy17hf33cb10171abe30eE,_unitas_runtime_audio_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_destroy17hccd340b3eaeb12a4E,_unitas_runtime_audio_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_load_clip17h7a66bdbb48ee0a80E,_unitas_runtime_audio_load_clip"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_load_clip17h89c767fa5f3fd94aE,_unitas_runtime_audio_load_clip"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_load_clip");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi24unitas_runtime_audio_mix17h5888229b840a6340E,_unitas_runtime_audio_mix"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi24unitas_runtime_audio_mix17he815ea12f7771c97E,_unitas_runtime_audio_mix"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_mix");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_pause17h144c7d5c21656518E,_unitas_runtime_audio_pause"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_pause17h56499a95315998fcE,_unitas_runtime_audio_pause"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_pause");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_play17hb1fbb9b8ffdb8e7cE,_unitas_runtime_audio_play"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_play17h472eb636a472efbeE,_unitas_runtime_audio_play"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_play");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_audio_register_clip_samples17h3e43a93cc3d54a90E,_unitas_runtime_audio_register_clip_samples"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_audio_register_clip_samples17h5aa439a19ec640c8E,_unitas_runtime_audio_register_clip_samples"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_register_clip_samples");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_clip17h3d922ccc177fcb54E,_unitas_runtime_audio_set_clip"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_clip17h47735daf710e687eE,_unitas_runtime_audio_set_clip"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_clip");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_audio_set_custom_duration17he5c51ece10b9acfcE,_unitas_runtime_audio_set_custom_duration"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_audio_set_custom_duration17h77b28164b24b14edE,_unitas_runtime_audio_set_custom_duration"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_custom_duration");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_listener17hbb5f9fa3b8874baaE,_unitas_runtime_audio_set_listener"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_listener17h00c93cde22b7fd7fE,_unitas_runtime_audio_set_listener"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_listener");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_loop17h1bafbcfe2381f960E,_unitas_runtime_audio_set_loop"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_loop17h60a54e250f2b5bc6E,_unitas_runtime_audio_set_loop"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_loop");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_audio_set_min_max_distance17he16f97709790bb3dE,_unitas_runtime_audio_set_min_max_distance"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_audio_set_min_max_distance17he85083b848f0665fE,_unitas_runtime_audio_set_min_max_distance"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_min_max_distance");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_mute17h9c728d6b8bac20d5E,_unitas_runtime_audio_set_mute"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_mute17h529edbda0c264fffE,_unitas_runtime_audio_set_mute"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_mute");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_paused17h02b8bee917bd0b4fE,_unitas_runtime_audio_set_paused"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_paused17h76ca237128a67ef2E,_unitas_runtime_audio_set_paused"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_paused");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_set_pitch17hd00742fd7d1c74f9E,_unitas_runtime_audio_set_pitch"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_set_pitch17h6d647c5a5c327305E,_unitas_runtime_audio_set_pitch"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_pitch");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_position17h76c27528159d240cE,_unitas_runtime_audio_set_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_position17he57a1c8ba46a0c67E,_unitas_runtime_audio_set_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_priority17h7808675f0ab9e5a9E,_unitas_runtime_audio_set_priority"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_priority17h74ab7b3918b0d4cbE,_unitas_runtime_audio_set_priority"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_priority");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_audio_set_spatial_blend17hfc5ed28965fede3cE,_unitas_runtime_audio_set_spatial_blend"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_audio_set_spatial_blend17h571f6187186607a1E,_unitas_runtime_audio_set_spatial_blend"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_spatial_blend"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_time17h6317f6f27e082f31E,_unitas_runtime_audio_set_time"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_time17hdeb7c3ab26b966b6E,_unitas_runtime_audio_set_time"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_time");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_volume17h00d99496f092502cE,_unitas_runtime_audio_set_volume"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_volume17hcc6a5e41a12a77a6E,_unitas_runtime_audio_set_volume"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_volume");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_silence_reason17h653c259802f25bc6E,_unitas_runtime_audio_silence_reason"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_silence_reason17hf2cf8e51c09dad4bE,_unitas_runtime_audio_silence_reason"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_silence_reason"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_audio_source_config17h497adfb1e3c87fb0E,_unitas_runtime_audio_source_config"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_audio_source_config17h8478fb6b7fdd21eeE,_unitas_runtime_audio_source_config"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_source_config"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_state17h8a3b66428b4bd097E,_unitas_runtime_audio_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_state17h43de63e7068433f7E,_unitas_runtime_audio_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_stop17h19b30735a878ff63E,_unitas_runtime_audio_stop"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_stop17h48ac9cfa1e8c61f3E,_unitas_runtime_audio_stop"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_stop");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_time17h66d00c1ac9933e54E,_unitas_runtime_audio_time"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_time17h0fba2f6a2d924f1aE,_unitas_runtime_audio_time"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_time");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_unpause17h4247d514abdbad3cE,_unitas_runtime_audio_unpause"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_unpause17h6d3681c14c3e782bE,_unitas_runtime_audio_unpause"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_unpause");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_position17h38c0823214324f80E,_unitas_runtime_body_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_position17he48b61de94a56e47E,_unitas_runtime_body_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_body_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_velocity17h64cf01b10ec6f392E,_unitas_runtime_body_velocity"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_velocity17h637ea70dcda511b5E,_unitas_runtime_body_velocity"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_body_velocity");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_box_cast17h46cc27e2e5797746E,_unitas_runtime_box_cast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_box_cast17hec37e3191c6dd364E,_unitas_runtime_box_cast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_box_cast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_character_controller_config17h858d458aca3f84c4E,_unitas_runtime_character_controller_config"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_character_controller_config17hfe58278a2b2a3368E,_unitas_runtime_character_controller_config"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_controller_config");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_character_move17h0a541c4fe6efb8e1E,_unitas_runtime_character_move"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_character_move17h152cbba7d72a9397E,_unitas_runtime_character_move"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_move");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_character_teleport17h490475ea405a2019E,_unitas_runtime_character_teleport"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_character_teleport17h3c3729894d71d4fcE,_unitas_runtime_character_teleport"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_teleport");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_company_name17h7ffce40e983f9473E,_unitas_runtime_company_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_company_name17h688171605c5ea5fbE,_unitas_runtime_company_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_company_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_destroy17hafd12bfc38f235c6E,_unitas_runtime_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_destroy17h411926e0d6b7dd66E,_unitas_runtime_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_dont_destroy_on_load17h5356e00a33ce3e0cE,_unitas_runtime_dont_destroy_on_load"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_dont_destroy_on_load17h6650846d9062aa79E,_unitas_runtime_dont_destroy_on_load"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_dont_destroy_on_load"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_error_message17hfad71cc2f47fb4dcE,_unitas_runtime_error_message"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_error_message17h53166320031de597E,_unitas_runtime_error_message"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_error_message");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_fixed_timestep17hcdaec06f6bd77e43E,_unitas_runtime_fixed_timestep"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_fixed_timestep17h733b80e2098c017bE,_unitas_runtime_fixed_timestep"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_fixed_timestep");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_input_axis17h620df7be80e0860aE,_unitas_runtime_input_axis"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_input_axis17h1301438c9cc4ffbaE,_unitas_runtime_input_axis"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_axis");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_input_axis_raw17hfa29c90a4022a471E,_unitas_runtime_input_axis_raw"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_input_axis_raw17h1605a0a69044a391E,_unitas_runtime_input_axis_raw"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_axis_raw");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_button_state17h1e0318e1daeeed2aE,_unitas_runtime_input_button_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_button_state17h67b6e6d6f1455f64E,_unitas_runtime_input_button_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_button_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_event17h919788e6192ef938E,_unitas_runtime_input_key_event"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_event17hdc50442c8af6a444E,_unitas_runtime_input_key_event"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_key_event");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_state17hf163052780135dfbE,_unitas_runtime_input_key_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_state17hf3802befa11fd7abE,_unitas_runtime_input_key_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_key_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi39unitas_runtime_input_mouse_button_event17h0e5b667e20c5801dE,_unitas_runtime_input_mouse_button_event"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi39unitas_runtime_input_mouse_button_event17hcca3496d93e0dac0E,_unitas_runtime_input_mouse_button_event"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_mouse_button_event"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_input_mouse_state17hc973b175b5ee83aeE,_unitas_runtime_input_mouse_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_input_mouse_state17h3db0d201f8aaf054E,_unitas_runtime_input_mouse_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_mouse_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_input_pointer17h26580ef664dc8c30E,_unitas_runtime_input_pointer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_input_pointer17hb9da7b7ea433ab0fE,_unitas_runtime_input_pointer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_pointer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_pointer_move17h00dff179e64ee975E,_unitas_runtime_input_pointer_move"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_pointer_move17h4d91364adbcdfdf8E,_unitas_runtime_input_pointer_move"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_pointer_move");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_input_scroll17h826b524cb3e7724dE,_unitas_runtime_input_scroll"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_input_scroll17h969daaf2a7588d20E,_unitas_runtime_input_scroll"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_scroll");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_scroll_state17ha80debf32ed621d1E,_unitas_runtime_input_scroll_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_scroll_state17h4ae5d19afc8390ecE,_unitas_runtime_input_scroll_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_scroll_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_linecast17hcc98e55c641e65ecE,_unitas_runtime_linecast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_linecast17hb2f3745cecb5b17cE,_unitas_runtime_linecast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_linecast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_load_scene17hd0f4d1069493a393E,_unitas_runtime_load_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_load_scene17haec53d3eda4d5024E,_unitas_runtime_load_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_load_scene");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_load_scene_mode17h373fb8a245fb977dE,_unitas_runtime_load_scene_mode"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_load_scene_mode17hcd0a63d6232aa646E,_unitas_runtime_load_scene_mode"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_load_scene_mode");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi18unitas_runtime_new17h500e8e90f781e775E,_unitas_runtime_new"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi18unitas_runtime_new17hf57ff83793e2d6a6E,_unitas_runtime_new"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_new");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_overlap_sphere17hd18a7652b9ff38d8E,_unitas_runtime_overlap_sphere"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_overlap_sphere17hae31cb5ea3386237E,_unitas_runtime_overlap_sphere"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_overlap_sphere");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_path_buffer17h22e1d7dcc830c8e1E,_unitas_runtime_path_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_path_buffer17h2882b18befa133a6E,_unitas_runtime_path_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_path_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_product_name17h6c3cd8cd0e0c4615E,_unitas_runtime_product_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_product_name17h33fab6897faa8d75E,_unitas_runtime_product_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_product_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_project_version17h46ece640f5e853a9E,_unitas_runtime_project_version"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_project_version17h5c1c231a5fecf08dE,_unitas_runtime_project_version"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_project_version");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_query_buffer17hb7e3a9d317c0b889E,_unitas_runtime_query_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_query_buffer17h61f20a3b9554d776E,_unitas_runtime_query_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_query_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_raycast17h943ba67e21d47258E,_unitas_runtime_raycast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_raycast17hc19072f977e4c539E,_unitas_runtime_raycast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_raycast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_render_buffer17hdc78a6c775c44926E,_unitas_runtime_render_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_render_buffer17h07d2dfd9624f1db5E,_unitas_runtime_render_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_create17h4f918fbffd66933aE,_unitas_runtime_render_camera_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_create17h74c93aa017256a63E,_unitas_runtime_render_camera_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi48unitas_runtime_render_camera_screen_point_to_ray17hc5680d2ee756f94fE,_unitas_runtime_render_camera_screen_point_to_ray"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi48unitas_runtime_render_camera_screen_point_to_ray17hdc0fdd41765078baE,_unitas_runtime_render_camera_screen_point_to_ray"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_screen_point_to_ray");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_update17hd0740b485405ba05E,_unitas_runtime_render_camera_update"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_update17h7e156c7ea756ecd4E,_unitas_runtime_render_camera_update"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_update"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi50unitas_runtime_render_camera_world_to_screen_point17hfb993e056515fcabE,_unitas_runtime_render_camera_world_to_screen_point"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi50unitas_runtime_render_camera_world_to_screen_point17h28cf3ff0e42da369E,_unitas_runtime_render_camera_world_to_screen_point"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_world_to_screen_point");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_render_frame17hec3659fd612e98fdE,_unitas_runtime_render_frame"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_render_frame17h50ffb5b30f2cb6dbE,_unitas_runtime_render_frame"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_frame");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_create17hed19f256020c462cE,_unitas_runtime_render_light_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_create17h5525a9de8b7e8897E,_unitas_runtime_render_light_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_light_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_update17hb06967f389e5c125E,_unitas_runtime_render_light_update"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_update17hc214e8355b73814aE,_unitas_runtime_render_light_update"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_light_update"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_material_create17h5246d796f654a6c5E,_unitas_runtime_render_material_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_material_create17h81257495b0cbb0a7E,_unitas_runtime_render_material_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_material_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_render_material_set_color17hb1e299efbc0e4addE,_unitas_runtime_render_material_set_color"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_render_material_set_color17h6425d482ba8545f1E,_unitas_runtime_render_material_set_color"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_material_set_color");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_render_mesh_create17h5747db34d9425dbaE,_unitas_runtime_render_mesh_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_render_mesh_create17hf2e45a910d54801fE,_unitas_runtime_render_mesh_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_mesh_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_mesh_destroy17h2fe2d544dfa62e2fE,_unitas_runtime_render_mesh_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_mesh_destroy17hcdfbe559a02fb0bfE,_unitas_runtime_render_mesh_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_mesh_destroy"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_renderer_create17hd26dda126fb87003E,_unitas_runtime_render_renderer_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_renderer_create17h1170d921a89158a2E,_unitas_runtime_render_renderer_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_renderer_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_render_renderer_set_enabled17hd55b5aad296e31a3E,_unitas_runtime_render_renderer_set_enabled"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_render_renderer_set_enabled17hb28f14345c97b450E,_unitas_runtime_render_renderer_set_enabled"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_renderer_set_enabled");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_resources_scan17h2c2035a85bb975baE,_unitas_runtime_resources_scan"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_resources_scan17hfb40d0562fcb82f0E,_unitas_runtime_resources_scan"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_resources_scan");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_scene_count17hb3fec7610ecc53d1E,_unitas_runtime_scene_count"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_scene_count17h866b4db30cc77d40E,_unitas_runtime_scene_count"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_scene_count");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_scene_name17hdcf466915f53f84aE,_unitas_runtime_scene_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_scene_name17heb23762eae2bee8fE,_unitas_runtime_scene_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_scene_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_script_index_build17hca5d8fd7ba2a05ecE,_unitas_runtime_script_index_build"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_script_index_build17h77ed6223ea25cbcbE,_unitas_runtime_script_index_build"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_script_index_build");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_set_active_scene17h12d5b909599b9ed6E,_unitas_runtime_set_active_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_set_active_scene17h369736d092c937e6E,_unitas_runtime_set_active_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_active_scene");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_position17h42f1e2111c3db403E,_unitas_runtime_set_body_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_position17h9996b830c39b1729E,_unitas_runtime_set_body_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_body_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_velocity17h1f352fb1b1a36a18E,_unitas_runtime_set_body_velocity"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_velocity17hb0eaa36ca961a2e0E,_unitas_runtime_set_body_velocity"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_body_velocity");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_set_callbacks17hafacf6ab56e68df7E,_unitas_runtime_set_callbacks"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_set_callbacks17h67abf43f1613333cE,_unitas_runtime_set_callbacks"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_callbacks");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_set_time_scale17hf18824078cc1d069E,_unitas_runtime_set_time_scale"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_set_time_scale17h0be6390d40ee69cbE,_unitas_runtime_set_time_scale"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_time_scale");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_sphere_cast17he33488d2dbc1ec43E,_unitas_runtime_sphere_cast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_sphere_cast17h5e8ad4b53cbc57e0E,_unitas_runtime_sphere_cast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_sphere_cast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi19unitas_runtime_tick17ha516c4dbfb47d346E,_unitas_runtime_tick"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi19unitas_runtime_tick17hd4e89c6033a6ccf9E,_unitas_runtime_tick"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_tick");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_time_scale17h588eead39b8815b8E,_unitas_runtime_time_scale"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_time_scale17haab330ee4957c7c3E,_unitas_runtime_time_scale"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_time_scale");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_unload_scene17h4c67f10655b2f340E,_unitas_runtime_unload_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_unload_scene17h436b2848886b7ee4E,_unitas_runtime_unload_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_unload_scene");
}
