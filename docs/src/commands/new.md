# __orbit new__

## __NAME__

new - create a new ip

## __SYNOPSIS__

```
orbit new [options] <path>
```

## __DESCRIPTION__

This command will create a new ip project. The default destination path is
the vendor/library/name relative to the DEV_PATH. If the DEV_PATH is not
configured, then it will use the current working directory. Creating a new 
ip involves creating a manifest file `Orbit.toml` and initializing an empty
git repository.
  
Use `--to` to override the destination path. This path is not allowed to
exist unless `--force` is specified.
  
Copying from existing files can be achieved in two ways. The recommended way
is to configure templates, which can be viewed with `--list`. Using 
`--template` will import the files from the template's root directory when
creating a new ip. On the other hand, using `--from` will import files from 
that directory.
  
Upon creation of an ip or file, variable substitution is performed. Variable
substitution takes form as a template processor using known information
about orbit's state and injecting into templated files.
  
A new file is able to be generated from within an ip under development with
the `--file` flag. You can view available files for importing from a
particular template by combining options `--template` and `--list`. To use
a file from a template in creating a new file, specify the template and
the source file's relative path with `--template` and `--from`. You can
specify a source path not tied to a template by just using `--from`.
   
If `--from` is omitted when creating a file, an empty file will be created.

## __OPTIONS__

`--name <name>`  
      The ip name to create

## __EXAMPLES__

```
orbit new gates
orbit new ./projects/lab1 --name adder
```
