# Phase 1

- [ ] Implement vault operations
    - Indexing (DFS [wikipedia](https://en.wikipedia.org/wiki/Depth-first_search))
    - Types (look [here](#types))
- [ ] Git operations using git2


## Types

- `FsIndex` -> DFS indexed directory and file tree with a root and a hashmap of the tree
- `FsNode` -> Node either file or folder with the type, size, id and path relative to the `FsIndex.root`
- `Vault` -> Kinda like the old `FsIndex` but with more functionality like delete update, it should hold inside 
    it `FsIndex` and prob some other metadata for configs for that especific vault

