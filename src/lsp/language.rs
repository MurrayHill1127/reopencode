//! File extension → LSP language ID mapping.

use std::path::Path;

pub fn language_id(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "py" | "pyi" | "pyc" => Some("python"),
        "ts" | "mts" | "cts" => Some("typescript"),
        "tsx" => Some("typescriptreact"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "jsx" => Some("javascriptreact"),
        "go" => Some("go"),
        "rb" => Some("ruby"),
        "java" => Some("java"),
        "kt" | "kts" => Some("kotlin"),
        "cs" => Some("csharp"),
        "cpp" | "cxx" | "cc" | "c++" => Some("cpp"),
        "c" => Some("c"),
        "h" | "hpp" | "hh" | "hxx" => Some("cpp"),
        "lua" => Some("lua"),
        "swift" => Some("swift"),
        "dart" => Some("dart"),
        "ex" | "exs" => Some("elixir"),
        "erl" | "hrl" => Some("erlang"),
        "hs" | "lhs" => Some("haskell"),
        "sh" | "bash" | "zsh" | "ksh" => Some("shellscript"),
        "yaml" | "yml" => Some("yaml"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        "xml" => Some("xml"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "scss" => Some("scss"),
        "sass" => Some("sass"),
        "sql" => Some("sql"),
        "md" | "markdown" => Some("markdown"),
        "php" => Some("php"),
        "r" => Some("r"),
        "scala" => Some("scala"),
        "clj" | "cljs" | "cljc" => Some("clojure"),
        "svelte" => Some("svelte"),
        "vue" => Some("vue"),
        "tf" => Some("terraform"),
        "gleam" => Some("gleam"),
        "zig" => Some("zig"),
        "ml" | "mli" => Some("ocaml"),
        "fs" | "fsi" | "fsx" => Some("fsharp"),
        _ => None,
    }
}

/// Return the language server command + args for a given language ID, or None
/// if no server is configured / installed.
pub fn server_command(lang: &str) -> Option<Vec<String>> {
    match lang {
        "rust" => which("rust-analyzer").map(|p| vec![p]),
        "python" => which("pyright-langserver")
            .map(|p| vec![p, "--stdio".into()])
            .or_else(|| which("pylsp").map(|p| vec![p])),
        "typescript" | "typescriptreact" | "javascript" | "javascriptreact" => {
            which("typescript-language-server").map(|p| vec![p, "--stdio".into()])
        }
        "go" => which("gopls").map(|p| vec![p]),
        "lua" => which("lua-language-server").map(|p| vec![p]),
        "cpp" | "c" => which("clangd").map(|p| vec![p, "--background-index".into()]),
        "ruby" => which("ruby-lsp").map(|p| vec![p]),
        "kotlin" => None, // requires JDTLS setup; skip for now
        "yaml" => which("yaml-language-server")
            .map(|p| vec![p, "--stdio".into()]),
        "shellscript" => which("bash-language-server")
            .map(|p| vec![p, "start".into()]),
        _ => None,
    }
}

fn which(cmd: &str) -> Option<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let candidate = std::path::PathBuf::from(dir).join(cmd);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rust_extension() {
        assert_eq!(language_id(&PathBuf::from("main.rs")), Some("rust"));
    }

    #[test]
    fn python_extension() {
        assert_eq!(language_id(&PathBuf::from("app.py")), Some("python"));
    }

    #[test]
    fn typescript_extension() {
        assert_eq!(language_id(&PathBuf::from("foo.ts")), Some("typescript"));
        assert_eq!(language_id(&PathBuf::from("comp.tsx")), Some("typescriptreact"));
    }

    #[test]
    fn unknown_extension() {
        assert_eq!(language_id(&PathBuf::from("file.xyz")), None);
    }

    #[test]
    fn no_extension() {
        assert_eq!(language_id(&PathBuf::from("Makefile")), None);
    }
}
