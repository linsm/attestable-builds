# The enclave container

The enclave container is what gets finally compiled into the .eif file.

Folder structure:
- `content/` all content for the container; will first be copied to the build dir and then included from there
- `dist/` temporary build dir, ignore 
