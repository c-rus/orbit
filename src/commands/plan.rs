use crate::Command;
use crate::FromCli;
use crate::interface::cli::Cli;
use crate::interface::arg::{Flag, Optional};
use crate::interface::errors::CliError;
use crate::core::context::Context;
use std::ffi::OsString;
use std::io::Write;
use crate::core::fileset::Fileset;
use crate::core::vhdl::vhdl::Identifier;

#[derive(Debug, PartialEq)]
pub struct Plan {
    plugin: Option<String>,
    bench: Option<Identifier>,
    top: Option<Identifier>,
    clean: bool,
    build_dir: Option<String>,
    filesets: Option<Vec<Fileset>>
}

impl Command for Plan {
    type Err = Box<dyn std::error::Error>;
    fn exec(&self, c: &Context) -> Result<(), Self::Err> {
        // check that user is in an IP directory
        c.goto_ip_path()?;
        // set top-level environment variables (@TODO verify these are valid toplevels to be set!)
        if let Some(t) = &self.top {
            std::env::set_var("ORBIT_TOP", t.to_string());
        }
        if let Some(b) = &self.bench {
            std::env::set_var("ORBIT_BENCH", b.to_string());
        }
        // determine the build directory
        let b_dir = if let Some(dir) = &self.build_dir {
            dir
        } else {
            c.get_build_dir()
        };
        // find plugin filesets
        let plug_fset = if let Some(plug) = &self.plugin {
            Some(c.get_plugins().get(plug).expect(&format!("plugin {} does not exist", plug)).filesets())
        } else {
            None
        };
        // @TODO pass in the current IP struct
        self.run(b_dir, plug_fset)
    }
}

use crate::core::vhdl::parser;
use crate::util::graph::Graph;
use crate::util::anyerror::AnyError;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct HashNode {
    index: usize,
    entity: parser::Entity,
    files: Vec<String>,
}

impl HashNode {
    pub fn index(&self) -> usize {
        self.index
    }
    
    fn new(entity: parser::Entity, index: usize, file: String) -> Self {
        let mut set = Vec::new();
        set.push(file);
        Self {
            entity: entity,
            index: index,
            files: set,
        }
    }

    fn add_file(&mut self, file: String) {
        if self.files.contains(&file) == false {
            self.files.push(file);
        }
    }
}

#[derive(Debug, PartialEq)]
struct ArchitectureFile {
    architecture: parser::Architecture,
    file: String,
}

#[derive(Debug, PartialEq)]
struct PackageFile {
    package: parser::Package,
    file: String,
}

#[derive(Debug, PartialEq)]
struct DesignUnit {
    name: Identifier,
    deps: Vec<parser::ResReference>,
    file: String,
}

impl Plan {

    pub fn build_graph(files: &Vec<String>) -> (Graph<Identifier, ()>, HashMap<Identifier, HashNode>) {
        // @TODO wrap graph in a hashgraph implementation
        let mut graph: Graph<Identifier, ()> = Graph::new();
        // entity identifier, HashNode (hash-node holds entity structs)
        let mut map = HashMap::<Identifier, HashNode>::new();

        let mut archs: Vec<ArchitectureFile> = Vec::new();
        let mut packs: Vec<PackageFile> = Vec::new();
        // read all files
        for source_file in files {
            if crate::core::fileset::is_vhdl(&source_file) == true {
                let contents = std::fs::read_to_string(&source_file).unwrap();
                let symbols = parser::VHDLParser::read(&contents).into_symbols();
                // add all entities to a graph and store architectures for later analysis
                let mut iter = symbols.into_iter().filter_map(|f| {
                    match f {
                        parser::VHDLSymbol::Entity(e) => Some(e),
                        parser::VHDLSymbol::Architecture(arch) => {
                            archs.push(ArchitectureFile{ architecture: arch, file: source_file.to_string() });
                            None
                        }
                        parser::VHDLSymbol::Package(pack) => {
                            packs.push(PackageFile{ package: pack, file: source_file.to_string() });
                            None
                        }
                        // @TODO link package body's to package declarations
                        _ => None,
                    }
                });
                while let Some(e) = iter.next() {
                    println!("entity external calls: {:?}", e.get_refs());
                    let index = graph.add_node(e.get_name().clone());
                    let hn = HashNode::new(e, index, source_file.to_string());
                    map.insert(graph.get_node(index).unwrap().clone(), hn);
                }
            }
        }

        println!("packages--- {:?}", packs);

        // go through all architectures and make the connections
        let mut archs = archs.into_iter();
        while let Some(af) = archs.next() {
            // link to the owner and add architecture's source file
            let entity_node = map.get_mut(&af.architecture.entity()).unwrap();
            entity_node.add_file(af.file);
            // create edges
            for dep in af.architecture.edges() {
                // verify the dep exists
                if let Some(node) = map.get(dep) {
                    graph.add_edge(node.index(), map.get(af.architecture.entity()).unwrap().index(), ());
                }
            }
        }
        (graph, map)
    }

    fn run(&self, build_dir: &str, plug_filesets: Option<&Vec<Fileset>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut build_path = std::env::current_dir().unwrap();
        build_path.push(build_dir);
        
        // check if to clean the directory
        if self.clean == true && std::path::Path::exists(&build_path) == true {
            std::fs::remove_dir_all(&build_path)?;
        }

        // gather filesets
        let files = crate::core::fileset::gather_current_files(&std::env::current_dir().unwrap());
        // build graph and map storage
        let (g, map) = Self::build_graph(&files);

        let mut bench = if let Some(t) = &self.bench {
            match map.get(&t) {
                Some(node) => {
                    if node.entity.is_testbench() == false {
                        return Err(AnyError(format!("entity \'{}\' is not a testbench and cannot be bench; use --top", t)))?
                    }
                    Some(node.index())
                },
                None => return Err(AnyError(format!("no entity named \'{}\'", t)))?
            }
        } else if self.top.is_none() {
            // filter to display tops that have ports (not testbenches)
            match g.find_root() {
                Ok(n) => Some(n),
                Err(e) => {
                    match e.len() {
                        0 => None,
                        _ => {
                            // gather all identifier names
                            let mut testbenches = e
                                .into_iter()
                                .map(|f| { g.get_node(f).unwrap() });
                            let mut err_msg = String::from("multiple testbenches were found:\n");
                            while let Some(tb) = testbenches.next() {
                                err_msg.push_str(&format!("\t{}\n", tb));
                            }
                            return Err(AnyError(err_msg))?;
                        }
                    }   
                }
            }
        } else {
            None // still could possibly be found by top level is top is some
        };

        // determine the top-level node index
        let top = if let Some(t) = &self.top {
            match map.get(&t) {
                Some(node) => {
                    if node.entity.is_testbench() == true {
                        return Err(AnyError(format!("entity \'{}\' is a testbench and cannot be top; use --bench", t)))?
                    }
                    let n = node.index();
                    // try to detect top level testbench
                    if bench.is_none() {
                        // check if only 1 is a testbench
                        let benches: Vec<usize> =  g.successors(n)
                            .filter(|f| map.get(&g.get_node(*f).unwrap()).unwrap().entity.is_testbench() )
                            .collect();

                        bench = match benches.len() {
                            0 => None,
                            1 => Some(*benches.first().unwrap()),
                            _ => {
                                // gather all identifier names
                                let mut testbenches = benches
                                    .into_iter()
                                    .map(|f| { g.get_node(f).unwrap() });
                                let mut err_msg = String::from("multiple testbenches were found:\n");
                                while let Some(tb) = testbenches.next() {
                                    err_msg.push_str(&format!("\t{}\n", tb));
                                }
                                return Err(AnyError(err_msg))?;
                            }
                        };
                    }
                    n
                },
                None => return Err(AnyError(format!("no entity named \'{}\'", t)))?
            }
        } else {
            Self::detect_top(&g, bench)
        };
        // enable immutability
        let bench = bench;

        let top_name = g.get_node(top).unwrap().to_string();
        let bench_name = if let Some(n) = bench {
            g.get_node(n).unwrap().to_string()
        } else {
            String::new()
        };

        std::env::set_var("ORBIT_TOP", &top_name);
        std::env::set_var("ORBIT_BENCH", &bench_name);

        let highest_point = match bench {
            Some(b) => b,
            None => top
        };
        // compute minimal topological ordering
        let min_order = g.minimal_topological_sort(highest_point);

        let mut file_order = Vec::new();
        for i in &min_order {
            // access the node key
            let key = g.get_node(*i).unwrap();
            // access the files associated with this key
            let mut v: Vec<&String> = map.get(key).as_ref().unwrap().files.iter().collect();
            file_order.append(&mut v);
        }

        // store data in blueprint TSV format
        let mut blueprint_data = String::new();

        // use command-line set filesets
        if let Some(fsets) = &self.filesets {
            for fset in fsets {
                let data = fset.collect_files(&files);
                for f in data {
                    blueprint_data += &format!("{}\t{}\t{}\n", fset.get_name(), std::path::PathBuf::from(f).file_stem().unwrap_or(&OsString::new()).to_str().unwrap(), f);
                }
            }
        }

        // collect data for the given plugin
        if let Some(fsets) = plug_filesets {
            // define pattern matching settings
            let match_opts = glob::MatchOptions {
                case_sensitive: false,
                require_literal_separator: false,
                require_literal_leading_dot: false,
            };
            // iterate through every collected file
            for file in &files {
                // check against every defined fileset for the plugin
                for fset in fsets {
                    if fset.get_pattern().matches_with(file, match_opts) == true {
                        // add to blueprint
                        blueprint_data += &fset.to_blueprint_string(file);
                    }
                }
            }
        }

        // collect in-order hdl data
        for file in file_order {
            if crate::core::fileset::is_rtl(&file) == true {
                blueprint_data += &format!("VHDL-RTL\twork\t{}\n", file);
            } else {
                blueprint_data += &format!("VHDL-SIM\twork\t{}\n", file);
            }
        }

        // create a output build directorie(s) if they do not exist
        if std::path::PathBuf::from(build_dir).exists() == false {
            std::fs::create_dir_all(build_dir).expect("could not create build dir");
        }
        // create the blueprint file
        let blueprint_path = build_path.join("blueprint.tsv");
        let mut blueprint_file = std::fs::File::create(&blueprint_path).expect("could not create blueprint.tsv file");
        // write the data
        blueprint_file.write_all(blueprint_data.as_bytes()).expect("failed to write data to blueprint");
        
        // create environment variables to .env file
        let env_path = build_path.join(".env");
        let mut env_file = std::fs::File::create(&env_path).expect("could not create .env file");
        let contents = format!("ORBIT_TOP={}\nORBIT_BENCH={}\n", &top_name, &bench_name);
        // write the data
        env_file.write_all(contents.as_bytes()).expect("failed to write data to .env file");

        // create a blueprint file
        println!("info: Blueprint created at: {}", blueprint_path.display());
        Ok(())
    }

    /// Given a `graph` and optionally a `bench`, detect the index corresponding
    /// to the top.
    /// 
    /// This function looks and checks if there is a single predecessor to the
    /// `bench` node.
    fn detect_top(graph: &Graph<Identifier, ()>, bench: Option<usize>) -> usize {
        if let Some(b) = bench {
            match graph.in_degree(b) {
                0 => panic!("no entities are tested in the testbench"),
                1 => graph.predecessors(b).next().unwrap(),
                _ => panic!("multiple tops are detected from testbench")
            }
        } else {
            todo!("find toplevel node that is not a bench")
        }
    }
}

impl FromCli for Plan {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self,  CliError<'c>> {
        cli.set_help(HELP);
        let command = Ok(Plan {
            top: cli.check_option(Optional::new("top").value("unit"))?,
            clean: cli.check_flag(Flag::new("clean"))?,
            bench: cli.check_option(Optional::new("bench").value("tb"))?,
            plugin: cli.check_option(Optional::new("plugin"))?,
            build_dir: cli.check_option(Optional::new("build-dir").value("dir"))?,
            filesets: cli.check_option_all(Optional::new("fileset").value("key=glob"))?,
        });
        command
    }
}

const HELP: &str = "\
Generates a blueprint file.

Usage:
    orbit plan [options]              

Options:
    --top <unit>            override auto-detected toplevel entity
    --bench <tb>            override auto-detected toplevel testbench
    --plugin <plugin>       collect filesets defined for a plugin
    --build-dir <dir>       set the output build directory
    --fileset <key=glob>... set an additional fileset
    --clean                 remove all files from the build directory
    --all                   include all found HDL files

Use 'orbit help plan' to learn more about the command.
";