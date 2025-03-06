# Template Module

This module contains the core template processing functionality for Baker.

## Core Components

### `TemplateProcessor`

The `TemplateProcessor` is responsible for processing template files and directories. It handles:

- File path rendering using template variables
- Content rendering for template files
- Directory creation
- Handling template-specific file extensions (.baker.j2)

### `TemplateOperation`

`TemplateOperation` defines the various operations that can be performed during template processing:

- `Copy`: Copy a regular file
- `Write`: Write rendered content to a file
- `CreateDirectory`: Create a directory
- `Ignore`: Skip a file or directory that matches an ignore pattern

## Processing Flow

1. The processor takes a file or directory from the template
2. It renders the path, replacing any template variables
3. It determines the operation type based on file type and extensions
4. It returns a `TemplateOperation` that can be executed

## File Extensions

Files with the `.baker.j2` extension are treated as template files and their content is rendered.
The `.baker.j2` suffix is removed in the final output.
