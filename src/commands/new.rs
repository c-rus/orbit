use crate::Command;
use crate::FromCli;
use crate::core::catalog::Catalog;
use crate::core::manifest::IpManifest;
use crate::core::variable::VariableTable;
use crate::interface::cli::Cli;
use crate::interface::arg::{Positional, Optional, Flag};
use crate::interface::errors::CliError;
use crate::core::pkgid;
use crate::interface::arg::Arg;
use crate::core::context::Context;
use crate::util::environment::Environment;
use std::error::Error;
use crate::util::anyerror::AnyError;
use crate::core::template::Template;

#[derive(Debug, PartialEq)]
pub struct New {
    ip: pkgid::PkgId,
    rel_path: Option<std::path::PathBuf>,
    template: Option<String>,
    list: bool,
}

impl FromCli for New {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self,  CliError<'c>> {
        cli.set_help(HELP);
        let command = Ok(New {
            rel_path: cli.check_option(Optional::new("path"))?,
            list: cli.check_flag(Flag::new("list"))?,
            template: cli.check_option(Optional::new("template").value("alias"))?,
            ip: cli.require_positional(Positional::new("ip"))?,
        });
        command
    }
}

impl Command for New {
    type Err = Box<dyn Error>;
    fn exec(&self, c: &Context) -> Result<(), Self::Err> {
        // view templates
        if self.list == true {
            println!("{}", Template::list_templates(&c.get_templates().values().into_iter().collect::<Vec<&Template>>()));
            return Ok(())
        }

        // extra validation for a new IP spec to contain all fields (V.L.N)
        if let Err(e) = self.ip.fully_qualified() {
            return Err(CliError::BadType(Arg::Positional(Positional::new("ip")), e.to_string()))?
        }
        let root = c.get_development_path().unwrap();

        // verify the pkgid is not taken
        {
            let catalog = Catalog::new()
                .development(c.get_development_path().unwrap())?
                .installations(c.get_cache_path())?
                .available(c.get_vendors())?;
            if catalog.inner().contains_key(&self.ip) == true {
                return Err(AnyError(format!("ip pkgid '{}' already taken", self.ip)))?
            }
        }

        // verify the template exists
        let template = if let Some(alias) = &self.template {
            match c.get_templates().get(alias) {
                Some(t) => Some(t),
                None => return Err(AnyError(format!("template '{}' does not exist", alias)))?
            }
        } else {
            None
        };

        // load variables
        let vars = VariableTable::new()
            .load_context(&c)?
            .load_pkgid(&self.ip)?
            .load_environment(&Environment::new().from_config(c.get_config())?)?;
        // only pass in necessary variables from context
        self.run(root, c.force, template, &vars)
    }
}

impl New {
    fn run(&self, root: &std::path::PathBuf, force: bool, template: Option<&Template>, lut: &VariableTable) -> Result<(), Box<dyn Error>> {
        // create ip stemming from ORBIT_PATH with default /VENDOR/LIBRARY/NAME
        let ip_path = if self.rel_path.is_none() {
            root.join(self.ip.get_vendor().as_ref().unwrap())
                .join(self.ip.get_library().as_ref().unwrap())
                .join(self.ip.get_name())
        } else {
            root.join(self.rel_path.as_ref().unwrap())
        };

        // verify the ip would exist alone on this path (cannot nest IPs)
        {
            // go to the very tip existing component of the path specified
            let mut path_clone = ip_path.clone();
            while path_clone.exists() == false {
                path_clone.pop();
            }
            // verify there are no current IPs living on this path
            if let Some(other_path) = Context::find_ip_path(&path_clone) {
                return Err(AnyError(format!("an IP already exists at path {}", other_path.display())))?
            }
        }

        let ip = IpManifest::create(ip_path, &self.ip, force, false)?;
        let root = ip.get_root();

        // import template if found
        if let Some(t) = template {
            // create hashmap to store variables
            t.import(&root, &lut)?;
        }

        // @TODO issue warning if the ip path is outside of the dev path or dev path is not set
        println!("info: new ip created at {}", root.display());
        Ok(())
    }
}

const HELP: &str = "\
Create a new orbit ip package.

Usage:
    orbit new [options] <ip>

Args:
    <ip>                the V.L.N for the new package (pkgid)

Options:
    --path <path>       set the destination directory
    --template <alias>  specify a template to import
    --list              view available templates

Use 'orbit help new' to read more about the command.
";