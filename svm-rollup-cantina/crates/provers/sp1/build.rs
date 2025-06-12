use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    println!("cargo::rerun-if-env-changed=SKIP_GUEST_BUILD");
    println!("cargo::rerun-if-env-changed=OUT_DIR");

    if matches!(
        std::env::var("SKIP_GUEST_BUILD")
            .as_ref()
            .map(|arg0: &String| String::as_str(arg0)),
        Ok("1") | Ok("true")
    ) {
        println!("cargo:warning=Skipping sp1 guest build");
        return;
    }

    let mut features = Vec::new();

    if cfg!(feature = "bench") {
        features.push("bench".to_string());
    }

    let build_args = BuildArgs {
        features,
        ..Default::default()
    };

    build_program_with_args("./guest-mock", build_args.clone());
    build_program_with_args("./guest-celestia", build_args.clone());
    build_program_with_args("./guest-solana", build_args);
}
