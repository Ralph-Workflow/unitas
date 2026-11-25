fn main() {
    let device = std::env::var("CARGO_FEATURE_DEVICE").is_ok();
    export(device);
}

fn export(device: bool) {
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_active_scene_index17ha148e49966465882E,_unitas_runtime_active_scene_index"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_active_scene_index17h86392782eba6cdfaE,_unitas_runtime_active_scene_index"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_active_scene_index");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_animator_attach17h2e97564c0c053a33E,_unitas_runtime_animator_attach"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_animator_attach17ha4bad9eaec9c27e1E,_unitas_runtime_animator_attach"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_attach");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_animator_controller_create17hb2acd60a389e6ae1E,_unitas_runtime_animator_controller_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_animator_controller_create17h774f1bbb36a66d64E,_unitas_runtime_animator_controller_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_controller_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_animator_controller_destroy17he5e35ec0f18a6305E,_unitas_runtime_animator_controller_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_animator_controller_destroy17h32ef9489b57b9109E,_unitas_runtime_animator_controller_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_controller_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_get_bool17hf15fb4813516747eE,_unitas_runtime_animator_get_bool"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_get_bool17h207c22aed2522552E,_unitas_runtime_animator_get_bool"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_bool");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_float17h61b5a1de6bfe836cE,_unitas_runtime_animator_get_float"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_float17h67e18971f1b0c4ccE,_unitas_runtime_animator_get_float"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_float");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_get_int17h2905f69db550dac3E,_unitas_runtime_animator_get_int"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_get_int17ha80c6f7912acfa85E,_unitas_runtime_animator_get_int"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_int");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_speed17hc4040954175f23b6E,_unitas_runtime_animator_get_speed"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_get_speed17h25e455b13eed3f78E,_unitas_runtime_animator_get_speed"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_get_speed");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_animator_reset_trigger17h7b8552358ad9748cE,_unitas_runtime_animator_reset_trigger"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_animator_reset_trigger17hf52a124380849d75E,_unitas_runtime_animator_reset_trigger"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_reset_trigger"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_set_bool17hce738f4f6262df77E,_unitas_runtime_animator_set_bool"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_animator_set_bool17hd4f5639ca526f3baE,_unitas_runtime_animator_set_bool"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_bool");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_animator_set_controller17h9435fbf194983f29E,_unitas_runtime_animator_set_controller"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_animator_set_controller17h2ca18222d8e9d429E,_unitas_runtime_animator_set_controller"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_controller"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_float17h6b32b4fb0952e892E,_unitas_runtime_animator_set_float"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_float17h4decf0fbdc4038b9E,_unitas_runtime_animator_set_float"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_float");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_set_int17h3254265ee0b99622E,_unitas_runtime_animator_set_int"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_animator_set_int17he53249e8aa17a293E,_unitas_runtime_animator_set_int"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_int");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_speed17h307629597b610e1cE,_unitas_runtime_animator_set_speed"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_animator_set_speed17h42c5cb347acdb960E,_unitas_runtime_animator_set_speed"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_speed");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_animator_set_trigger17h4a74963bb0c2d3ddE,_unitas_runtime_animator_set_trigger"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_animator_set_trigger17h62752949ea42dbe9E,_unitas_runtime_animator_set_trigger"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_set_trigger"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_info17h0d44eaf5fd101270E,_unitas_runtime_animator_state_info"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_info17h158f3445565392fcE,_unitas_runtime_animator_state_info"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_state_info"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_name17hc9cb348634b998cbE,_unitas_runtime_animator_state_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_animator_state_name17hf0d74d267e2f5c0eE,_unitas_runtime_animator_state_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_animator_state_name"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_clear_listener17he2d6a99bbb59d391E,_unitas_runtime_audio_clear_listener"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_clear_listener17h2a9fb3799bb584beE,_unitas_runtime_audio_clear_listener"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_clear_listener"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_clip_info17h8545ff4c1a06586cE,_unitas_runtime_audio_clip_info"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_clip_info17h092a5bbbe21172afE,_unitas_runtime_audio_clip_info"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_clip_info");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_audio_create17h96b353576a3a8319E,_unitas_runtime_audio_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_audio_create17h59980960fd7cc9bfE,_unitas_runtime_audio_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_destroy17hbf406f9320991b2dE,_unitas_runtime_audio_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_destroy17h5600b5b7deee15e1E,_unitas_runtime_audio_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_load_clip17he40167a3d1a9c3b9E,_unitas_runtime_audio_load_clip"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_load_clip17h7f7fecbfdb41bf02E,_unitas_runtime_audio_load_clip"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_load_clip");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi24unitas_runtime_audio_mix17hf1fd80e8ca69a5f0E,_unitas_runtime_audio_mix"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi24unitas_runtime_audio_mix17h43444e37e0392b1bE,_unitas_runtime_audio_mix"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_mix");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_pause17h45a31ca3e676dc9bE,_unitas_runtime_audio_pause"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_pause17h9caa810b1ef01ce4E,_unitas_runtime_audio_pause"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_pause");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_play17h5dcd53f88a5a5b07E,_unitas_runtime_audio_play"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_play17hfe27e2a30a65bb9aE,_unitas_runtime_audio_play"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_play");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_audio_register_clip_samples17h6b005aba54a02d2eE,_unitas_runtime_audio_register_clip_samples"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_audio_register_clip_samples17hdf1141dd140bc7d9E,_unitas_runtime_audio_register_clip_samples"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_register_clip_samples");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_clip17hbe6ccf7c8259b9f8E,_unitas_runtime_audio_set_clip"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_clip17h867893155c756660E,_unitas_runtime_audio_set_clip"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_clip");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_audio_set_custom_duration17h4607e7cebbbd0d1aE,_unitas_runtime_audio_set_custom_duration"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_audio_set_custom_duration17h68a8a81a6d75cbe9E,_unitas_runtime_audio_set_custom_duration"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_custom_duration");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_listener17h8e830cc7c87da590E,_unitas_runtime_audio_set_listener"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_listener17h71620f5db7fb6159E,_unitas_runtime_audio_set_listener"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_listener");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_loop17h6a392afcd8117ea9E,_unitas_runtime_audio_set_loop"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_loop17h2a8cce7750e981cdE,_unitas_runtime_audio_set_loop"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_loop");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_audio_set_min_max_distance17h8434db28afe5503fE,_unitas_runtime_audio_set_min_max_distance"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi41unitas_runtime_audio_set_min_max_distance17hecbccd1bfdc4723cE,_unitas_runtime_audio_set_min_max_distance"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_min_max_distance");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_mute17h7e2ad32d300ea497E,_unitas_runtime_audio_set_mute"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_mute17h94da89f3880bc141E,_unitas_runtime_audio_set_mute"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_mute");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_paused17h119fd8c0cd03bf18E,_unitas_runtime_audio_set_paused"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_paused17hf5252e8093a3b65dE,_unitas_runtime_audio_set_paused"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_paused");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_set_pitch17h557b244e2bda1063E,_unitas_runtime_audio_set_pitch"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_audio_set_pitch17h2f3c266c2f3a5251E,_unitas_runtime_audio_set_pitch"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_pitch");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_position17hf0a81e3d70c3fee1E,_unitas_runtime_audio_set_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_position17h75edb25ba986ecadE,_unitas_runtime_audio_set_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_priority17h3bbff2892b8b25eeE,_unitas_runtime_audio_set_priority"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_audio_set_priority17h14b3156fdd4f1eeaE,_unitas_runtime_audio_set_priority"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_priority");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_audio_set_spatial_blend17h6a0dff58ddcf03a2E,_unitas_runtime_audio_set_spatial_blend"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi38unitas_runtime_audio_set_spatial_blend17h4b08cebb6e975775E,_unitas_runtime_audio_set_spatial_blend"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_spatial_blend"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_time17h6d628035ec14c2a9E,_unitas_runtime_audio_set_time"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_audio_set_time17h9f7a8be102911c44E,_unitas_runtime_audio_set_time"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_time");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_volume17ha0beb512bd714e6dE,_unitas_runtime_audio_set_volume"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_audio_set_volume17hdd3912bc1d1e25a4E,_unitas_runtime_audio_set_volume"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_set_volume");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_silence_reason17hdcd475e661c9d175E,_unitas_runtime_audio_silence_reason"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_audio_silence_reason17hd25a17d25696e47fE,_unitas_runtime_audio_silence_reason"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_silence_reason"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_audio_source_config17hf444ff9d3dba7495E,_unitas_runtime_audio_source_config"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_audio_source_config17hbb9c1081d032b7f0E,_unitas_runtime_audio_source_config"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_source_config"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_state17h6a11228dad0e45c7E,_unitas_runtime_audio_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_audio_state17h2a2efba0faae7915E,_unitas_runtime_audio_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_stop17hcf69b983e2a203e6E,_unitas_runtime_audio_stop"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_stop17h86cc401128d6357eE,_unitas_runtime_audio_stop"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_stop");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_time17h9986e243883cb6ceE,_unitas_runtime_audio_time"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_audio_time17he3f0748f6d4809c5E,_unitas_runtime_audio_time"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_time");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_unpause17hb531ec2ec8c9d189E,_unitas_runtime_audio_unpause"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_audio_unpause17ha72729894c8b4eabE,_unitas_runtime_audio_unpause"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_audio_unpause");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_position17h33bebd38b3174641E,_unitas_runtime_body_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_position17h5eee68b9678ba59cE,_unitas_runtime_body_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_body_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_velocity17h80d5ca82bc73394cE,_unitas_runtime_body_velocity"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_body_velocity17hc56d399bd823db6eE,_unitas_runtime_body_velocity"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_body_velocity");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_box_cast17hff088d0369dced7eE,_unitas_runtime_box_cast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_box_cast17he6f9727e0a0b5793E,_unitas_runtime_box_cast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_box_cast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_character_controller_config17h92e7603057f1452bE,_unitas_runtime_character_controller_config"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_character_controller_config17he7118928dc4b3ad1E,_unitas_runtime_character_controller_config"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_controller_config");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_character_move17h629e273560dddbbbE,_unitas_runtime_character_move"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_character_move17h10e4f7d4a5ba213cE,_unitas_runtime_character_move"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_move");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_character_teleport17hebe2a77a98af17d1E,_unitas_runtime_character_teleport"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_character_teleport17h562f033944610ff8E,_unitas_runtime_character_teleport"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_character_teleport");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_company_name17h530999f7f3dc3e5aE,_unitas_runtime_company_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_company_name17h7323fdf5098fac3bE,_unitas_runtime_company_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_company_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_destroy17h8dfc221d163e42aeE,_unitas_runtime_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_destroy17he965de652b1ef6bbE,_unitas_runtime_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_destroy");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_dont_destroy_on_load17hc2b9faedda6fff39E,_unitas_runtime_dont_destroy_on_load"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_dont_destroy_on_load17haed3dca6fa5f0709E,_unitas_runtime_dont_destroy_on_load"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_dont_destroy_on_load"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_error_message17hb409846e3f031c65E,_unitas_runtime_error_message"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_error_message17h5a63d186e115bff1E,_unitas_runtime_error_message"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_error_message");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_fixed_timestep17hd51edf2704622982E,_unitas_runtime_fixed_timestep"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_fixed_timestep17h58e62b94c1ac7a6eE,_unitas_runtime_fixed_timestep"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_fixed_timestep");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_input_axis17hde275d35ee7d2f6bE,_unitas_runtime_input_axis"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_input_axis17hd1d67586fc58daefE,_unitas_runtime_input_axis"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_axis");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_input_axis_raw17h27377fac3f4eb264E,_unitas_runtime_input_axis_raw"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_input_axis_raw17h4f767c578a329a86E,_unitas_runtime_input_axis_raw"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_axis_raw");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_button_state17hdacd43ca37228c20E,_unitas_runtime_input_button_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_button_state17h514e79f23e6085ddE,_unitas_runtime_input_button_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_button_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_event17hf491c43a9405749dE,_unitas_runtime_input_key_event"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_event17hb14474fd2c305759E,_unitas_runtime_input_key_event"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_key_event");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_state17hb4c88dd5e0cf6968E,_unitas_runtime_input_key_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_input_key_state17he893b9d4d7d970c4E,_unitas_runtime_input_key_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_key_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi39unitas_runtime_input_mouse_button_event17h17eb457f121f4fc2E,_unitas_runtime_input_mouse_button_event"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi39unitas_runtime_input_mouse_button_event17h666cf04ec2137396E,_unitas_runtime_input_mouse_button_event"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_mouse_button_event"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_input_mouse_state17h9c4a279c59f6f3d7E,_unitas_runtime_input_mouse_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_input_mouse_state17hde41b0b2e0c7a1c2E,_unitas_runtime_input_mouse_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_mouse_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_input_pointer17h05067910d5147b6cE,_unitas_runtime_input_pointer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_input_pointer17h6c8d9ba2cddbcf0fE,_unitas_runtime_input_pointer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_pointer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_pointer_move17hfd058909ccb94e5dE,_unitas_runtime_input_pointer_move"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_pointer_move17h7ba58a1913ee0175E,_unitas_runtime_input_pointer_move"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_pointer_move");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_input_scroll17h16c1704150d8d288E,_unitas_runtime_input_scroll"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_input_scroll17hc957d5631d939824E,_unitas_runtime_input_scroll"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_scroll");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_scroll_state17hbb33325d76ae417bE,_unitas_runtime_input_scroll_state"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_input_scroll_state17h49995848bfb7ac3eE,_unitas_runtime_input_scroll_state"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_input_scroll_state");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_linecast17h7ee75f9d3e4692d6E,_unitas_runtime_linecast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi23unitas_runtime_linecast17h7200f784c4df6a43E,_unitas_runtime_linecast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_linecast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_load_scene17h85cb64420ad8d049E,_unitas_runtime_load_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_load_scene17h55d77c90f37173d1E,_unitas_runtime_load_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_load_scene");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_load_scene_mode17h2f22323151cbe9c6E,_unitas_runtime_load_scene_mode"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_load_scene_mode17h1b8154427021fadfE,_unitas_runtime_load_scene_mode"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_load_scene_mode");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi18unitas_runtime_new17h2fdd7fa67ed54071E,_unitas_runtime_new"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi18unitas_runtime_new17h2db93cdaf9783b2aE,_unitas_runtime_new"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_new");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_overlap_sphere17hb89d53c926278e0fE,_unitas_runtime_overlap_sphere"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_overlap_sphere17hcb01ecc07e9c582fE,_unitas_runtime_overlap_sphere"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_overlap_sphere");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_path_buffer17h9cfa30757c8f236eE,_unitas_runtime_path_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_path_buffer17h90c2127857abb87eE,_unitas_runtime_path_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_path_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_product_name17h6cf85b1c7c22d3b9E,_unitas_runtime_product_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_product_name17hd956b83051a7d944E,_unitas_runtime_product_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_product_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_project_version17hba5c7e01078940e8E,_unitas_runtime_project_version"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi30unitas_runtime_project_version17h9c5300ad579519b2E,_unitas_runtime_project_version"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_project_version");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_query_buffer17ha4ed888b3334e8c1E,_unitas_runtime_query_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_query_buffer17hdb0a2453d7c99b94E,_unitas_runtime_query_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_query_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_raycast17h9bb216701d993a38E,_unitas_runtime_raycast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi22unitas_runtime_raycast17h40e6e1da78c940a7E,_unitas_runtime_raycast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_raycast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_render_buffer17h2d0a1fb3b6763160E,_unitas_runtime_render_buffer"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_render_buffer17h83e2ab3e1642c099E,_unitas_runtime_render_buffer"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_buffer");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_create17h9d8557009059abbfE,_unitas_runtime_render_camera_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_create17h5bfbe7c80acd2ba8E,_unitas_runtime_render_camera_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi48unitas_runtime_render_camera_screen_point_to_ray17hb607f8ed092c334bE,_unitas_runtime_render_camera_screen_point_to_ray"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi48unitas_runtime_render_camera_screen_point_to_ray17h16381e3b171f4231E,_unitas_runtime_render_camera_screen_point_to_ray"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_screen_point_to_ray");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_update17h3ed35cfe24db2310E,_unitas_runtime_render_camera_update"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi35unitas_runtime_render_camera_update17h96f845f9861086fdE,_unitas_runtime_render_camera_update"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_update"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi50unitas_runtime_render_camera_world_to_screen_point17h2bd5bab0fdec5f28E,_unitas_runtime_render_camera_world_to_screen_point"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi50unitas_runtime_render_camera_world_to_screen_point17hd036d9e53b4a1313E,_unitas_runtime_render_camera_world_to_screen_point"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_camera_world_to_screen_point");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_render_frame17h4aeec426a6a41730E,_unitas_runtime_render_frame"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_render_frame17hba3efbec17772297E,_unitas_runtime_render_frame"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_frame");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_create17h68ecb80639252c27E,_unitas_runtime_render_light_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_create17had3291fb590e4382E,_unitas_runtime_render_light_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_light_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_update17heaa4a784cdf6d549E,_unitas_runtime_render_light_update"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_light_update17hf7bed82a8e7d60b7E,_unitas_runtime_render_light_update"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_light_update"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_material_create17h81ebb10ae15caf51E,_unitas_runtime_render_material_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_material_create17h7c985b214443b872E,_unitas_runtime_render_material_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_material_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_render_material_set_color17h0c30da986792a6ffE,_unitas_runtime_render_material_set_color"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi40unitas_runtime_render_material_set_color17h909c1fca1dedc935E,_unitas_runtime_render_material_set_color"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_material_set_color");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_render_mesh_create17hd34e68559f925de6E,_unitas_runtime_render_mesh_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_render_mesh_create17ha0e36286e58c740bE,_unitas_runtime_render_mesh_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_mesh_create");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_mesh_destroy17h58fa5cbe44b35531E,_unitas_runtime_render_mesh_destroy"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi34unitas_runtime_render_mesh_destroy17h7c3f28fe7b2f097fE,_unitas_runtime_render_mesh_destroy"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_mesh_destroy"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_renderer_create17hcacf3a6202721718E,_unitas_runtime_render_renderer_create"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi37unitas_runtime_render_renderer_create17h42362ca5cdf4a042E,_unitas_runtime_render_renderer_create"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_renderer_create"
    );
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_render_renderer_set_enabled17hbb1ffa1b12ba1f8aE,_unitas_runtime_render_renderer_set_enabled"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi42unitas_runtime_render_renderer_set_enabled17h4a519961eaa75111E,_unitas_runtime_render_renderer_set_enabled"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_render_renderer_set_enabled");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_resources_scan17hc2d5a09b4c186440E,_unitas_runtime_resources_scan"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_resources_scan17hc92fc53f426f0168E,_unitas_runtime_resources_scan"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_resources_scan");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_scene_count17hda01d72c6538504aE,_unitas_runtime_scene_count"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_scene_count17he651a719c2ced379E,_unitas_runtime_scene_count"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_scene_count");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_scene_name17h2d758a6fbfea845dE,_unitas_runtime_scene_name"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_scene_name17h47e9a1f28757aca1E,_unitas_runtime_scene_name"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_scene_name");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_script_index_build17h094f498bd0718d9aE,_unitas_runtime_script_index_build"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi33unitas_runtime_script_index_build17h605db787643892d1E,_unitas_runtime_script_index_build"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_script_index_build");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_set_active_scene17hc7f763cdde06d7f6E,_unitas_runtime_set_active_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi31unitas_runtime_set_active_scene17hd5b6394703d80318E,_unitas_runtime_set_active_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_active_scene");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_position17h436eba6c46cfe4ddE,_unitas_runtime_set_body_position"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_position17hb65a1c0c7a462f84E,_unitas_runtime_set_body_position"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_body_position");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_velocity17hce20683b5b210dc3E,_unitas_runtime_set_body_velocity"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi32unitas_runtime_set_body_velocity17h7e044a9eee611b6fE,_unitas_runtime_set_body_velocity"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_body_velocity");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_set_callbacks17hfedd88b5d0728ff2E,_unitas_runtime_set_callbacks"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi28unitas_runtime_set_callbacks17h40541309baf80e62E,_unitas_runtime_set_callbacks"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_callbacks");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_set_time_scale17h69741c95b61173e9E,_unitas_runtime_set_time_scale"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi29unitas_runtime_set_time_scale17hce4bf6f279394defE,_unitas_runtime_set_time_scale"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_set_time_scale");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_sphere_cast17h7a657925ef61b058E,_unitas_runtime_sphere_cast"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi26unitas_runtime_sphere_cast17h682be4543d9c9ae6E,_unitas_runtime_sphere_cast"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_sphere_cast");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi19unitas_runtime_tick17h69d5f7edf343f969E,_unitas_runtime_tick"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi19unitas_runtime_tick17h6777f5dc4791cc59E,_unitas_runtime_tick"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_tick");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_time_scale17hd6085ad12bc689ffE,_unitas_runtime_time_scale"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi25unitas_runtime_time_scale17hda4adb0524b091f1E,_unitas_runtime_time_scale"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_time_scale");
    let alias = if device {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_unload_scene17hba8f5ae90986faa6E,_unitas_runtime_unload_scene"
    } else {
        "-Wl,-alias,__ZN14unitas_runtime3abi27unitas_runtime_unload_scene17h1777a4cbec6390f0E,_unitas_runtime_unload_scene"
    };
    println!("cargo:rustc-cdylib-link-arg={alias}");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_runtime_unload_scene");
}
