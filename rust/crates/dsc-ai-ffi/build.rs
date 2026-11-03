// DLLの窓口（src/lib.rs）と境界の型（dsc-ai/src/abi.rs）から、Unity用のC#宣言を生成する。

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=../dsc-ai/src/abi.rs");

    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .input_extern_file("../dsc-ai/src/abi.rs")
        .csharp_dll_name("dsc_ai_native")
        .csharp_namespace("Dsc.Ai.Native")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("public")
        // dsc_ai_create は u8 で受け取るので、関数の引数からは見つからない
        .always_included_types(["Difficulty"])
        .generate_csharp_file("../../../Assets/Code/Cpu/Native/NativeMethods.g.cs")
        .expect("failed to generate C# bindings");
}
