/**
 * Composable for building directory structures from vault/local data
 * Shared between files.ts and localFiles.ts stores
 */
import { Folder, type FolderFile } from '~/models/Folder';
import { getArchiveAddress, isArchivePrivate } from '~/utils/archive';
import type { IArchive, IFileMetadata, ArchiveAccess } from '~/types/folder';

/**
 * Archive structure for directory building
 */
export interface BuildableArchive {
  name: string;
  archive_access: ArchiveAccess;
  files: Array<{
    path: string;
    metadata?: {
      uploaded?: number;
      created?: number;
      modified?: number;
      size?: number;
    };
    file_access?: { Private: number[] | string } | { Public: string };
    access_data?: { Private: number[] | string } | { Public: string };
  }>;
}

/**
 * Individual file for directory building
 */
export interface BuildableFile {
  path: string;
  name?: string;
  metadata?: {
    uploaded?: number;
    created?: number;
    modified?: number;
    size?: number;
  };
  file_access?: { Private: number[] | string } | { Public: string };
  access_data?: { Private: number[] | string } | { Public: string };
  file_type?: 'Public' | 'Private';
  is_loaded?: boolean;
  is_private?: boolean;
}

/**
 * Options for building directory structure
 */
export interface DirectoryBuilderOptions {
  rootName?: string;
  logPrefix?: string;
  /** For vault files, use access_data. For local files, use file_access */
  accessDataField?: 'access_data' | 'file_access';
  /** Whether files should be marked as loaded by default */
  filesLoadedByDefault?: boolean;
}

const DEFAULT_OPTIONS: DirectoryBuilderOptions = {
  rootName: 'Vault',
  logPrefix: '',
  accessDataField: 'access_data',
  filesLoadedByDefault: false
};

/**
 * Build a file object for adding to a folder
 */
function buildFileObject(
  file: BuildableFile,
  archiveName: string,
  archiveAccess: ArchiveAccess | undefined,
  isPrivate: boolean,
  options: DirectoryBuilderOptions
): Omit<FolderFile, 'name' | 'parent'> {
  const accessData = file.access_data || file.file_access;

  return {
    path: file.path || file.name || '',
    metadata: file.metadata || { size: 0, uploaded: 0, created: 0, modified: 0 },
    file_access: file.file_access,
    access_data: accessData,
    archive_access: archiveAccess,
    archive_name: archiveName,
    is_loaded: file.is_loaded ?? options.filesLoadedByDefault ?? !!accessData,
    is_loading: false,
    load_error: false,
    type: isPrivate ? 'private_file' : 'public_file'
  };
}

/**
 * Add a file to the directory structure, creating intermediate folders as needed
 */
function addFileToDirectory(
  root: Folder,
  filePath: string,
  fileData: Omit<FolderFile, 'name' | 'parent'>
): void {
  const fileParts = filePath.split('/').filter((part) => part.length > 0);
  let current: Folder = root;

  fileParts.forEach((part, index) => {
    if (index === fileParts.length - 1) {
      // This is the file - add to current folder
      current.addFile({
        ...fileData,
        name: part,
        parent: null // Will be set by addFile
      });
    } else {
      // This is a subdirectory - create or get it
      let subFolder = current.getSubfolder(part);
      if (!subFolder) {
        subFolder = new Folder(part, current);
        current.addSubfolder(subFolder);
      }
      current = subFolder;
    }
  });
}

/**
 * Process an archive and add its files to the directory
 */
function processArchive(
  rootDirectory: Folder,
  archive: BuildableArchive,
  archiveIndex: number,
  options: DirectoryBuilderOptions
): void {
  const hasName = archive.name && archive.name.trim() !== '';
  const isPrivate = isArchivePrivate(archive as IArchive);
  const archiveAddress = getArchiveAddress(archive as IArchive);

  if (!hasName) {
    // Unnamed archive - add files directly to root
    archive.files.forEach((file) => {
      const fileData = buildFileObject(
        file as BuildableFile,
        archive.name || `archive_${archiveIndex}`,
        archive.archive_access,
        isPrivate,
        options
      );
      addFileToDirectory(rootDirectory, file.path, fileData);
    });
  } else {
    // Named archive - create archive folder with unique name if needed
    let archiveFolderName = archive.name;
    let counter = 1;

    // Handle duplicate archive names
    while (rootDirectory.getChild(archiveFolderName)) {
      const existingChild = rootDirectory.getChild(archiveFolderName);
      // If it's the same archive (same address), don't create a duplicate
      if (existingChild && existingChild instanceof Folder && existingChild.archive) {
        const existingAddress = getArchiveAddress(existingChild.archive as IArchive);
        if (existingAddress === archiveAddress) {
          return; // Same archive, skip
        }
      }
      archiveFolderName = `${archive.name} (${counter})`;
      counter++;
    }

    const archiveFolder = new Folder(
      archiveFolderName,
      rootDirectory,
      null,
      true,
      archive as IArchive
    );
    rootDirectory.addSubfolder(archiveFolder);

    // Add files within the archive folder
    archive.files.forEach((file) => {
      const fileData = buildFileObject(
        file as BuildableFile,
        archive.name,
        archive.archive_access,
        isPrivate,
        options
      );
      addFileToDirectory(archiveFolder, file.path, fileData);
    });
  }
}

/**
 * Process individual files (not in archives)
 */
function processIndividualFiles(
  rootDirectory: Folder,
  files: BuildableFile[],
  options: DirectoryBuilderOptions
): void {
  files.forEach((file) => {
    const filePath = file.path || file.name || '';
    const isPrivate = file.file_type === 'Private' || file.is_private === true;

    const fileData = buildFileObject(
      file,
      '', // No archive
      undefined,
      isPrivate,
      options
    );

    addFileToDirectory(rootDirectory, filePath, fileData);
  });
}

/**
 * Build a root directory from archives and files
 * @param archives - Array of archives with their files
 * @param files - Array of individual files (not in archives)
 * @param options - Build options
 * @returns The root directory, or null if no content
 */
export function buildRootDirectory(
  archives: BuildableArchive[],
  files: BuildableFile[] = [],
  options: DirectoryBuilderOptions = {}
): Folder | null {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const prefix = opts.logPrefix ? `>>> ${opts.logPrefix} ` : '>>> ';

  try {
    // Check if there's any content
    if (!archives?.length && !files?.length) {
      return null;
    }

    console.log(`${prefix}Building directory structure...`);
    const rootDirectory = new Folder(opts.rootName!);

    // Process archives
    archives.forEach((archive, index) => {
      processArchive(rootDirectory, archive, index, opts);
    });

    // Process individual files
    if (files?.length) {
      processIndividualFiles(rootDirectory, files, opts);
    }

    return rootDirectory;
  } catch (error) {
    console.log(`${prefix}ERROR: Failed to build directory structure`, error);
    return null;
  }
}

/**
 * Composable for directory building in stores
 */
export function useDirectoryBuilder(options: DirectoryBuilderOptions = {}) {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  /**
   * Build directory from vault/local structure
   */
  const build = (
    archives: BuildableArchive[],
    files: BuildableFile[] = []
  ): Folder | null => {
    return buildRootDirectory(archives, files, opts);
  };

  /**
   * Rebuild with new data
   */
  const rebuild = (
    currentRoot: Folder | null,
    archives: BuildableArchive[],
    files: BuildableFile[] = []
  ): Folder | null => {
    return buildRootDirectory(archives, files, opts);
  };

  return {
    build,
    rebuild
  };
}
