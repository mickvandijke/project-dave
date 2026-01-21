/**
 * Folder model class for representing directory structures
 * Used by both files.ts and localFiles.ts stores
 */
import type { IArchive, ArchiveAccess } from '~/types/folder';

/**
 * File item within a folder
 */
export interface FolderFile {
  name: string;
  path: string;
  parent: Folder | null;
  metadata?: {
    uploaded?: number;
    created?: number;
    modified?: number;
    size?: number;
  };
  file_access?: { Private: number[] | string } | { Public: string };
  access_data?: { Private: number[] | string } | { Public: string };
  archive_access?: ArchiveAccess;
  archive_name?: string;
  is_loaded?: boolean;
  is_loading?: boolean;
  load_error?: boolean;
  type?: 'public_file' | 'private_file';
}

/**
 * Child item in a folder - can be either a Folder or a FolderFile
 */
export type FolderChild = Folder | FolderFile;

/**
 * Check if a child item is a Folder
 */
export function isFolder(child: FolderChild): child is Folder {
  return child instanceof Folder;
}

/**
 * Folder class representing a directory in the file tree
 */
export class Folder {
  name: string;
  paths: unknown;
  parent: Folder | null;
  children: FolderChild[] = [];
  isArchive: boolean = false;
  archive?: IArchive;

  constructor(
    name: string,
    parent: Folder | null = null,
    paths: unknown = null,
    isArchive: boolean = false,
    archive?: IArchive
  ) {
    this.name = name;
    this.parent = parent;
    this.isArchive = isArchive;
    this.archive = archive;
    this.paths = paths;
  }

  /**
   * Add a subfolder to this folder
   * @param subfolder - The subfolder to add
   * @throws Error if subfolder with same name already exists
   */
  addSubfolder(subfolder: Folder): void {
    try {
      const existing = this.children.find(
        (child) => isFolder(child) && child.name === subfolder.name
      );

      if (existing) {
        throw new Error('Subfolder already exists');
      }

      subfolder.parent = this;
      this.children.push(subfolder);
    } catch (error) {
      console.log('>>> ERROR: Failed to add subfolder', error);
    }
  }

  /**
   * Add a file to this folder
   * @param file - The file to add
   */
  addFile(file: FolderFile): void {
    try {
      const existingFile = this.children.find(
        (child) => !isFolder(child) && child.name === file.name
      ) as FolderFile | undefined;

      if (existingFile) {
        // Update existing file if new one has more data
        if (file.is_loaded && !existingFile.is_loaded) {
          Object.assign(existingFile, file);
        }
        return;
      }

      file.parent = this;
      this.children.push(file);
    } catch (error) {
      console.log('>>> ERROR: Failed to add file', error);
    }
  }

  /**
   * Get the parent folder
   * @returns The parent folder, or null if this is the root
   */
  getParent(): Folder | null {
    return this.parent;
  }

  /**
   * Get a child by name (subfolder or file)
   * @param name - The name to search for
   * @returns The child item, or undefined if not found
   */
  getChild(name: string): FolderChild | undefined {
    return this.children.find((child) => child.name === name);
  }

  /**
   * Get a child subfolder by name
   * @param name - The subfolder name
   * @returns The subfolder, or undefined if not found
   */
  getSubfolder(name: string): Folder | undefined {
    const child = this.getChild(name);
    return child && isFolder(child) ? child : undefined;
  }

  /**
   * Get a child file by name
   * @param name - The file name
   * @returns The file, or undefined if not found
   */
  getFile(name: string): FolderFile | undefined {
    const child = this.getChild(name);
    return child && !isFolder(child) ? child : undefined;
  }

  /**
   * Check if this folder has any children
   */
  get isEmpty(): boolean {
    return this.children.length === 0;
  }

  /**
   * Get all subfolders
   */
  get subfolders(): Folder[] {
    return this.children.filter(isFolder) as Folder[];
  }

  /**
   * Get all files
   */
  get files(): FolderFile[] {
    return this.children.filter((child) => !isFolder(child)) as FolderFile[];
  }

  /**
   * Get the full path from root to this folder
   * @returns Array of folder names from root to this folder
   */
  getPath(): string[] {
    const path: string[] = [];
    let current: Folder | null = this;

    while (current) {
      path.unshift(current.name);
      current = current.parent;
    }

    return path;
  }

  /**
   * Get the path as a string
   * @param separator - Path separator (default: '/')
   */
  getPathString(separator: string = '/'): string {
    return this.getPath().join(separator);
  }

  /**
   * Find a nested folder by path
   * @param pathParts - Array of folder names to navigate
   * @returns The folder at the path, or null if not found
   */
  findByPath(pathParts: string[]): Folder | null {
    if (pathParts.length === 0) {
      return this;
    }

    const [first, ...rest] = pathParts;
    const child = this.getSubfolder(first);

    if (!child) {
      return null;
    }

    return child.findByPath(rest);
  }

  /**
   * Get or create a nested folder path
   * @param pathParts - Array of folder names to create
   * @returns The folder at the end of the path
   */
  getOrCreatePath(pathParts: string[]): Folder {
    if (pathParts.length === 0) {
      return this;
    }

    const [first, ...rest] = pathParts;
    let child = this.getSubfolder(first);

    if (!child) {
      child = new Folder(first, this);
      this.addSubfolder(child);
    }

    return child.getOrCreatePath(rest);
  }
}

/**
 * Create a root folder with the given name
 * @param name - The root folder name
 * @returns A new Folder instance
 */
export function createRootFolder(name: string = 'Root'): Folder {
  return new Folder(name);
}
