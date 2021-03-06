use std::collections::BTreeMap;

use crate::Command;
use crate::FromCli;
use crate::core::catalog::Catalog;
use crate::core::catalog::IpLevel;
use crate::core::pkgid::PkgId;
use crate::core::version::AnyVersion;
use crate::core::version::Version;
use crate::core::vhdl::primaryunit::PrimaryUnit;
use crate::interface::cli::Cli;
use crate::interface::arg::{Positional, Flag, Optional};
use crate::interface::errors::CliError;
use crate::core::context::Context;
use crate::util::anyerror::AnyError;
use crate::util::anyerror::Fault;

#[derive(Debug, PartialEq)]
pub struct Probe {
    ip: PkgId,
    tags: bool,
    units: bool,
    version: Option<AnyVersion>,
    changelog: bool,
    readme: bool,
}

impl FromCli for Probe {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self,  CliError<'c>> {
        cli.set_help(HELP);
        let command = Ok(Probe {
            tags: cli.check_flag(Flag::new("tags"))?,
            units: cli.check_flag(Flag::new("units"))?,
            changelog: cli.check_flag(Flag::new("changes"))?,
            readme: cli.check_flag(Flag::new("readme"))?,
            version: cli.check_option(Optional::new("ver").switch('v'))?,
            ip: cli.require_positional(Positional::new("ip"))?,
        });
        command
    }
}

impl Command for Probe {
    type Err = Fault;
    fn exec(&self, c: &Context) -> Result<(), Self::Err> {

        // gather the catalog (all manifests)
        let mut catalog = Catalog::new()
            .store(c.get_store_path())
            .development(c.get_development_path().unwrap())?
            .installations(c.get_cache_path())?
            .available(c.get_vendors())?;

        let ids = catalog.inner().keys().map(|f| { f }).collect();
        let target = crate::core::ip::find_ip(&self.ip, ids)?;
        // ips under this key
        let status = catalog.inner_mut().remove(&target).unwrap();

        // collect all ip in the user's universe to see if ip exists
        if self.tags == true {
            println!("{}", format_version_table(status));
            return Ok(())
        }

        // find most compatible version with the partial version
        let v = self.version.as_ref().unwrap_or(&AnyVersion::Latest);

        let ip = match status.get(v, false) {
            Some(i) => i,
            None => Err(AnyError(format!("ip '{}' is not found as version '{}'", target, v)))?
        };

        if self.units == true {
            let units = match status.get(v, true).is_some() {
                true => ip.collect_units(),
                false => ip.read_units().unwrap_or(Vec::new()),
            };
            println!("{}", format_units_table(units));
            return Ok(())
        }

        println!("{}", ip);
        self.run()
    }
}

impl Probe {
    fn run(&self) -> Result<(), Fault> {
        Ok(())
    }
}

/// Creates a string for to display the primary design units for the particular ip.
fn format_units_table(table: Vec<PrimaryUnit>) -> String {
    let header = format!("\
{:<32}{:<14}{:<9}
{:->32}{3:->14}{3:->9}\n",
                "Identifier", "Unit", "Public", " ");
    let mut body = String::new();

    let mut table = table;
    table.sort_by(|a, b| a.as_iden().unwrap().cmp(b.as_iden().unwrap()));
    for unit in table {
        body.push_str(&format!("{:<32}{:<14}{:<2}\n", 
            unit.as_iden().unwrap().to_string(), 
            unit.to_string(), 
            "y"));
    }
    header + &body
}

/// Creates a string for a version table for the particular ip.
fn format_version_table(table: IpLevel) -> String {
    let header = format!("\
{:<15}{:<9}
{:->15}{2:->9}\n",
                "Version", "Status", " ");
    // create a hashset of all available versions to form a list
    let mut btmap = BTreeMap::<&Version, (bool, bool, bool)>::new();
    // log what version the dev ip is at
    if let Some(ip) = table.get_dev() {
        btmap.insert(ip.get_version(), (true, false, false));
    }
    // log the installation versions
    for ip in table.get_installations() {
        match btmap.get_mut(&ip.get_version()) {
            Some(entry) => entry.1 = true,
            None => { btmap.insert(ip.get_version(), (false, true, false)); () },
        }
    }
    // log the available versions
    for ip in table.get_availability() {
        match btmap.get_mut(&ip.get_version()) {
            Some(entry) => entry.2 = true,
            None => { btmap.insert(ip.get_version(), (false, false, true)); () },
        } 
    }
    // create body text
    let mut body = String::new();
    for (ver, status) in btmap.iter().rev() {
        body.push_str(&format!("{:<15}{:<2}{:<2}{:<2}\n", 
            ver.to_string(),
            { if status.0 { "D" } else { "" } },
            { if status.1 { "I" } else { "" } },
            { if status.2 { "A" } else { "" } },
        ));
    }
    header + &body
}

const HELP: &str = "\
Access information about an ip

Usage:
    orbit probe [options] <ip>

Args:
    <ip>               the pkgid to request data about

Options:
    --tags                      display the list of possible versions
    --range <version:version>   narrow the displayed version list
    --ver, -v <version>         select a particular existing ip version
    --units                     display primary design units within an ip
    --changes                   view the changelog
    --readme                    view the readme

Use 'orbit help query' to learn more about the command.
";