use clif::cmd::{FromCli, Command};
use clif::Cli;
use clif::arg::{Positional, Flag};
use clif::Error as CliError;
use crate::core::context::Context;
use crate::util::anyerror::Fault;
use std::collections::BTreeMap;
use crate::OrbitResult;
use crate::core::pkgid::PkgPart;

use crate::core::v2::catalog::Catalog;
use crate::core::version::AnyVersion;
use crate::core::v2::catalog::IpLevel;

#[derive(Debug, PartialEq)]
pub struct Search {
    ip: Option<PkgPart>,
    cached: bool,
    developing: bool,
    queued: bool,
    available: bool,
}

impl Command<Context> for Search {
    type Status = OrbitResult;

    fn exec(&self, c: &Context) -> Self::Status {

        let default = !(self.cached || self.developing || self.available || self.queued);
        let mut catalog = Catalog::new();

        // collect development IP
        // if default || self.developing { catalog = catalog.development(c.get_development_path().unwrap())?; }
        
        // collect installed IP
        if default || self.cached { catalog = catalog.installations(c.get_cache_path())?; }

        // collect downloaded IP
        if default || self.queued { catalog = catalog.queue(c.get_queue_path())?; }

        // collect available IP
       //  if default || self.available { catalog = catalog.available(c.get_vendors())?; }

        self.run(&catalog)
    }
}

impl Search {
    fn run(&self, catalog: &Catalog) -> Result<(), Fault> {

        // transform into a BTreeMap for alphabetical ordering
        let mut tree = BTreeMap::new();
        catalog.inner()
            .into_iter()
            // filter by name if user entered a pkgid to search
            .filter(|(key, _)| {
                match &self.ip {
                    Some(pkgid) => key.to_string().contains(pkgid.as_ref()),
                    None => true,
                }
            })
            .for_each(|(key, status)| {
                tree.insert(key, status);
            });

        println!("{}", Self::fmt_table(tree));
        Ok(())
    }

    fn fmt_table(catalog: BTreeMap<&PkgPart, &IpLevel>) -> String {
        let header = format!("\
{:<28}{:<10}{:<9}
{3:->28}{3:->10}{3:->11}\n", 
            "Package", "Latest", "Status", " ");
        let mut body = String::new();
        for (ip, status) in catalog {
            body.push_str(&format!("{:<28}{:<10}     {:<9}\n", 
                ip.to_string(),
                status.get(&AnyVersion::Latest, false).unwrap().get_man().get_ip().get_version(),
                if status.is_installed() == true {
                    "Installed"
                } else if status.is_queued() == true {
                    "Downloaded"
                } else {
                    ""
                },
            ));
        }
        header + &body
    }
}

impl FromCli for Search {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self,  CliError> {
        cli.check_help(clif::Help::new().quick_text(HELP).ref_usage(2..4))?;
        let command = Ok(Search {
            ip: cli.check_positional(Positional::new("ip"))?,
            queued: cli.check_flag(Flag::new("download").switch('d'))?,
            cached: cli.check_flag(Flag::new("install").switch('i'))?,
            developing: cli.check_flag(Flag::new("develop"))?,
            available: cli.check_flag(Flag::new("available").switch('a'))?,
        });
        command
    }
}

const HELP: &str = "\
Browse and find ip from the catalog.

Usage:
    orbit search [options] [<ip>]

Args:
    <ip>                a partially qualified pkgid to lookup ip

Options:
    --install, -i       filter for ip installed to cache
    --download, -d      filter for ip downloaded to the queue
    --develop           filter for ip in-development
    --available, -a     filter for ip available from vendors

Use 'orbit help search' to learn more about the command.
";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fmt_table() {
        let t = Search::fmt_table(BTreeMap::new());
        let table = "\
Package                     Latest    Status   
--------------------------- --------- ---------- 
";
        assert_eq!(t, table);
    }
}