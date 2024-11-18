{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust
    cargo rustc
    # LLVM
    # llvmPackages_16.libllvm
    # llvmPackages_16.libllvm.dev
    # Libraries for LLVM
    libffi libxml2
    ncurses
  ];

  # shellHook = ''
  #   export LLVM_SYS_160_PREFIX=${pkgs.llvmPackages_16.libllvm.dev}/bin
  # '';
}
