use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    ReadProject,
    EditContent,
    EditScripts,
    ExecuteProjectCode,
    ImportExternalFiles,
    ChangePackagesAndSettings,
    Build,
    Sign,
    Deploy,
    ExternalNetwork,
    Review,
}

impl Scope {
    pub const ALL: [Scope; 11] = [
        Scope::ReadProject,
        Scope::EditContent,
        Scope::EditScripts,
        Scope::ExecuteProjectCode,
        Scope::ImportExternalFiles,
        Scope::ChangePackagesAndSettings,
        Scope::Build,
        Scope::Sign,
        Scope::Deploy,
        Scope::ExternalNetwork,
        Scope::Review,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Scope::ReadProject => "read-project",
            Scope::EditContent => "edit-content",
            Scope::EditScripts => "edit-scripts",
            Scope::ExecuteProjectCode => "execute-project-code",
            Scope::ImportExternalFiles => "import-external-files",
            Scope::ChangePackagesAndSettings => "change-packages-and-settings",
            Scope::Build => "build",
            Scope::Sign => "sign",
            Scope::Deploy => "deploy",
            Scope::ExternalNetwork => "external-network",
            Scope::Review => "review",
        }
    }

    pub fn from_name(name: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|scope| scope.as_str() == name)
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Scope;

    #[test]
    fn scope_set_is_the_fixed_aut_007_set() {
        let names: Vec<&str> = Scope::ALL.iter().map(|scope| scope.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "read-project",
                "edit-content",
                "edit-scripts",
                "execute-project-code",
                "import-external-files",
                "change-packages-and-settings",
                "build",
                "sign",
                "deploy",
                "external-network",
                "review",
            ]
        );
        assert_eq!(Scope::ALL.len(), 11);
    }

    #[test]
    fn scope_names_round_trip() {
        for scope in Scope::ALL {
            assert_eq!(Scope::from_name(scope.as_str()), Some(scope));
        }
        assert_eq!(Scope::from_name("write-content"), None);
    }
}
