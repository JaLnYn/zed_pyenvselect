use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use zed_extension_api::{
    self as zed, SlashCommand, SlashCommandArgumentCompletion, SlashCommandOutput,
    SlashCommandOutputSection, Worktree,
};

struct PythonEnvironment {
    name: String,
    python_path: PathBuf,
}

struct PythonEnvironmentSelectExtension;

impl PythonEnvironmentSelectExtension {
    fn is_venv(path: &Path) -> bool {
        let activate_script = path.join("bin").join("activate");
        let pyvenv_cfg = path.join("pyvenv.cfg");

        activate_script.exists() || pyvenv_cfg.exists()
    }

    fn find_python_executable(venv_path: &Path) -> Option<PathBuf> {
        let python_path = venv_path.join("bin").join("python");
        if python_path.exists() {
            Some(python_path)
        } else {
            None
        }
    }

    fn find_venvs_from_worktree(_worktree: &Worktree) -> Vec<PythonEnvironment> {
        let root_path = PathBuf::from(_worktree.root_path());
        Self::find_venvs_rec(&root_path)
    }

    fn find_venvs_rec(dir: &Path) -> Vec<PythonEnvironment> {
        let mut venvs = Vec::new();

        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        if Self::is_venv(&path) {
                            if let Some(python_path) = Self::find_python_executable(&path) {
                                venvs.push(PythonEnvironment {
                                    name: path.file_name().unwrap().to_string_lossy().into_owned(),
                                    python_path,
                                });
                            }
                        } else {
                            // Recursively search subdirectories
                            venvs.extend(Self::find_venvs_rec(&path));
                        }
                    }
                }
            }
            Err(e) => {
                venvs.push(PythonEnvironment {
                    name: format!("Error reading directory ({}): {}", dir.display(), e),
                    python_path: PathBuf::new(),
                });
            }
        }
        venvs
    }

    fn find_envs_from_conda() -> Result<Vec<PythonEnvironment>, String> {
        let output = Command::new("conda")
            .args(&["info", "--envs"])
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Command executed with failing error code: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_conda_output(&output_str)
    }

    fn parse_conda_output(output: &str) -> Result<Vec<PythonEnvironment>, String> {
        let mut envs = Vec::new();
        let mut lines = output.lines();

        // Skip header lines
        while let Some(line) = lines.next() {
            if line.starts_with('#') {
                continue;
            }
            break;
        }

        // Parse environment lines
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let env_path = PathBuf::from(parts[1]);
                if let Some(python_path) = Self::find_python_executable(&env_path) {
                    envs.push(PythonEnvironment {
                        name: parts[0].to_string(),
                        python_path,
                    });
                }
            }
        }
        Ok(envs)
    }

    fn get_all_python_environments(&self, worktree: Option<&Worktree>) -> Vec<PythonEnvironment> {
        let mut environments = Vec::new();

        println!("here1");

        // Get virtual environments from worktree
        if let Some(worktree) = worktree {
            environments.extend(Self::find_venvs_from_worktree(worktree));
        }

        // Get Conda environments
        if let Ok(conda_envs) = Self::find_envs_from_conda() {
            environments.extend(conda_envs);
        }

        environments
    }
}

impl zed::Extension for PythonEnvironmentSelectExtension {
    fn new() -> Self {
        PythonEnvironmentSelectExtension
    }

    fn complete_slash_command_argument(
        &self,
        command: SlashCommand,
        _args: Vec<String>,
    ) -> Result<Vec<zed_extension_api::SlashCommandArgumentCompletion>, String> {
        match command.name.as_str() {
            "pyenvcur" => Ok(vec![]),
            "pyenvlst" => Ok(vec![]),
            "pyenvselect" => Ok(vec![]),
            command => Err(format!("unknown slash command: \"{command}\"")),
        }
    }

    fn run_slash_command(
        &self,
        command: SlashCommand,
        args: Vec<String>,
        _worktree: Option<&Worktree>,
    ) -> Result<SlashCommandOutput, String> {
        match command.name.as_str() {
            "pyenvcur" => {
                if args.is_empty() {
                    return Err("nothing to echo".to_string());
                }

                let text = args.join(" ");

                Ok(SlashCommandOutput {
                    sections: vec![SlashCommandOutputSection {
                        range: (0..text.len()).into(),
                        label: "Echo".to_string(),
                    }],
                    text,
                })
            }
            "pyenvlst" => {
                let all_envs = self.get_all_python_environments(_worktree);

                // Find the longest environment name for proper alignment
                let max_name_length = all_envs.iter().map(|env| env.name.len()).max().unwrap_or(0);

                // Format each environment with aligned columns
                let formatted_envs: Vec<String> = all_envs
                    .iter()
                    .map(|env| {
                        format!(
                            "{:<width$}    {}",
                            env.name,
                            env.python_path.display(),
                            width = max_name_length
                        )
                    })
                    .collect();

                let mut text = formatted_envs.join("\n");
                text = format!("======\n {} ======", text);
                text = format!("{}\nlen: {}", text, all_envs.len());

                Ok(SlashCommandOutput {
                    sections: vec![SlashCommandOutputSection {
                        range: (0..text.len()).into(),
                        label: "Python Environments".to_string(),
                    }],
                    text,
                })
            }
            "pyenvselect" => {
                if args.is_empty() {
                    return Err("nothing to echo".to_string());
                }

                let text = args.join(" ");

                Ok(SlashCommandOutput {
                    sections: vec![SlashCommandOutputSection {
                        range: (0..text.len()).into(),
                        label: "Echo".to_string(),
                    }],
                    text,
                })
            }
            command => Err(format!("unknown slash command: \"{command}\"")),
        }
    }
}

zed::register_extension!(PythonEnvironmentSelectExtension);
