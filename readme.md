# Sigil
~~Blazing fast~~ tool for programm construction, based on rewriting system that realise multilevel introspection. Or what lamers call dependently typed fp lang.
The intntion is to build a lang that is a mix of deptyped fp and imperative features. All of it should run on VM.
## Roadmap
### Stage 0.
1. [_] Finalise parser to match grammar spec
1. [_] TUI-esque CLI tool for assistance
    1. [_] Debugging
### Stage 1.
1. [_] Semantic analysis
    1. [_] Scoping check
    1. [_] Case analysis
    1. [_] Case coverage
1. [_] Separation analysis
    1. [_] Consteval spotting
1. [_] Admissible typechecking
### Stage 2.
1. [_] Codegen
    1. [_] Asyncification of imp code
    1. [_] Asyncification of fp code
1. [_] Runtime
1. [_] VM APIs
    1. [_] Networking
    1. [_] FS
    1. [_] IPC
    1. [_] Acceleration primitives (?)