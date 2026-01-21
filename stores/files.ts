import {invoke} from "@tauri-apps/api/core";
import type {IFolder, IFile, IVaultStructure, IArchive, IFailedArchive, IFileMetadata} from "~/types/folder";
import {useWalletStore} from "~/stores/wallet";
import { Folder } from "~/models/Folder";
import { getAddressKey } from "~/utils/archive";
import { generateTempCode } from "~/utils/archive";
import { buildRootDirectory, type BuildableArchive, type BuildableFile } from "~/composables/useDirectoryBuilder";

export const useFileStore = defineStore("files", () => {
    const walletStore = useWalletStore();
    const toast = useToast();

    // Archive loading state using simple refs (shared pattern from useArchiveLoading)
    const loadingArchiveAddresses = ref<Set<string>>(new Set());
    const loadingArchiveNames = ref<Map<string, string>>(new Map());

    const addLoadingArchive = (name: string, address: string, isPrivate: boolean) => {
        const key = getAddressKey(address, isPrivate);
        console.log('>>> Adding loading archive:', name, address, isPrivate ? 'Private' : 'Public');
        loadingArchiveAddresses.value.add(key);
        loadingArchiveNames.value.set(key, name);
        console.log('>>> Current loading addresses:', Array.from(loadingArchiveAddresses.value));
    };

    const removeLoadingArchive = (address: string, isPrivate: boolean) => {
        const key = getAddressKey(address, isPrivate);
        console.log('>>> Removing loading archive:', address, isPrivate ? 'Private' : 'Public');
        loadingArchiveAddresses.value.delete(key);
        loadingArchiveNames.value.delete(key);
        console.log('>>> Current loading addresses:', Array.from(loadingArchiveAddresses.value));
    };

    const clearAllLoadingArchives = () => {
        console.log('>>> Clearing all loading archives');
        loadingArchiveAddresses.value.clear();
        loadingArchiveNames.value.clear();
    };

    // State
    const files = ref<IFile[]>([]);
    const vaultStructure = ref<IVaultStructure | null>(null);
    const failedArchives = ref<IFailedArchive[]>([]);
    const rootDirectory = ref<IFolder | null>(null);
    const currentDirectory = ref<IFolder | null>(null);
    const pendingVaultStructure = ref(false);
    const loadedFiles = ref<Map<string, any>>(new Map());
    const currentLoadCode = ref<string | null>(null);

    // Computed
    const currentDirectoryFiles = computed(() => {
        if (!currentDirectory.value) {
            return [];
        }

        return Object.keys(currentDirectory.value);
    });

    // Create loading archives list from simple tracking
    const loadingArchives = computed(() => {
        return Array.from(loadingArchiveAddresses.value).map(address => ({
            name: loadingArchiveNames.value.get(address) || 'Unknown',
            archive_access: address.startsWith('private:') 
                ? { Private: address.substring(8) } 
                : { Public: address.substring(7) }
        }));
    });

    // Actions
    const rebuildRootDirectory = () => {
        try {
            // Reset rootDirectory
            rootDirectory.value = null;

            if (!vaultStructure.value?.archives.length && !vaultStructure.value?.files?.length) {
                return;
            }

            // Use shared directory builder
            const archives = vaultStructure.value.archives.map((archive: IArchive) => ({
                name: archive.name,
                archive_access: archive.archive_access,
                files: archive.files.map((file: IFileMetadata) => ({
                    path: file.path,
                    metadata: file.metadata,
                    access_data: file.access_data,
                    file_type: file.file_type,
                    is_loaded: file.is_loaded
                }))
            })) as BuildableArchive[];

            const individualFiles = (vaultStructure.value.files || []).map((file: IFileMetadata) => ({
                path: file.path,
                metadata: file.metadata,
                access_data: file.access_data,
                file_type: file.file_type,
                is_loaded: file.is_loaded
            })) as BuildableFile[];

            rootDirectory.value = buildRootDirectory(archives, individualFiles, {
                rootName: 'Vault',
                accessDataField: 'access_data'
            });

            // Set current directory
            currentDirectory.value = rootDirectory.value;
        } catch (error) {
            console.log(">>> ERROR: Failed to build archive-based local vault", error);
            rootDirectory.value = null;
        }
    };

    // Legacy alias for buildRootDirectory
    const buildRootDirectoryLegacy = () => {
        try {
            // Reset rootDirectory
            rootDirectory.value = null;

            if (!vaultStructure.value?.archives.length && !vaultStructure.value?.files?.length) {
                return;
            }

            console.log(">>> Building Archive-based Local Vault...");
            rootDirectory.value = new Folder("Vault");

            vaultStructure.value.archives.forEach((archive: IArchive, archiveIndex: number) => {
                // Check if archive has a name (not empty after sanitization)
                const hasName = archive.name && archive.name.trim() !== '';

                if (!hasName) {
                    // Unnamed archive - add files directly to root
                    archive.files.forEach((file: IFileMetadata) => {
                        const fileParts = file.path.split("/").filter(part => part.length > 0);
                        let current: Folder = rootDirectory.value!;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file - add directly to current folder
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.access_data,
                                    access_data: file.access_data,
                                    is_loaded: !!file.access_data,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name || `archive_${archiveIndex}`,
                                    archive_access: archive.archive_access,
                                    parent: null
                                });
                            } else {
                                // This is a subdirectory - create regular folder (not archive folder)
                                let subFolder = current.getSubfolder(part);
                                if (!subFolder) {
                                    subFolder = new Folder(part, current);
                                    current.addSubfolder(subFolder);
                                }
                                current = subFolder;
                            }
                        });
                    });
                } else {
                    // Named archive - create archive folder with unique name if needed
                    let archiveFolderName = archive.name;
                    let counter = 1;

                    // Handle duplicate archive names by checking if it's actually a different archive
                    while (rootDirectory.value!.getChild(archiveFolderName)) {
                        const existingChild = rootDirectory.value!.getChild(archiveFolderName);
                        // If it's the same archive (same address), don't create a duplicate
                        if (existingChild && existingChild instanceof Folder && existingChild.archive) {
                            const existingAddress = 'Private' in existingChild.archive.archive_access
                                ? existingChild.archive.archive_access.Private
                                : existingChild.archive.archive_access.Public;
                            const currentAddress = 'Private' in archive.archive_access
                                ? archive.archive_access.Private
                                : archive.archive_access.Public;
                            if (existingAddress === currentAddress) {
                                // Same archive, don't add counter - just skip creating a new folder
                                return;
                            }
                        }
                        archiveFolderName = `${archive.name} (${counter})`;
                        counter++;
                    }

                    const archiveFolder = new Folder(archiveFolderName, rootDirectory.value, null, true, archive);
                    rootDirectory.value!.addSubfolder(archiveFolder);

                    // Add files within the archive folder
                    archive.files.forEach((file: IFileMetadata) => {
                        const fileParts = file.path.split("/").filter(part => part.length > 0);
                        let current: Folder = archiveFolder;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.access_data,
                                    access_data: file.access_data,
                                    is_loaded: !!file.access_data,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name,
                                    archive_access: archive.archive_access,
                                    parent: null
                                });
                            } else {
                                // This is a subdirectory within the archive
                                // Allow duplicate directory names within different archives by not checking globally
                                let subFolder = current.getSubfolder(part);
                                if (!subFolder) {
                                    subFolder = new Folder(part, current);
                                    current.addSubfolder(subFolder);
                                }
                                current = subFolder;
                            }
                        });
                    });
                }
            });

            // Process individual files (not in archives)
            vaultStructure.value?.files?.forEach((file: IFileMetadata) => {
                const fileParts = file.path.split("/").filter(part => part.length > 0);
                let current: Folder = rootDirectory.value!;

                fileParts.forEach((part: string, index: number) => {
                    if (index === fileParts.length - 1) {
                        // This is the file - add directly to current folder
                        current.addFile({
                            path: file.path,
                            metadata: file.metadata,
                            file_access: file.access_data,
                            access_data: file.access_data,
                            is_loaded: file.is_loaded,
                            is_loading: false,
                            load_error: false,
                            name: part,
                            archive_name: "", // Individual files have no archive
                            parent: null
                        });
                    } else {
                        // This is a subdirectory - create regular folder (not archive folder)
                        let subFolder = current.getSubfolder(part);
                        if (!subFolder) {
                            subFolder = new Folder(part, current);
                            current.addSubfolder(subFolder);
                        }
                        current = subFolder;
                    }
                });
            });

            // Set current directory
            currentDirectory.value = rootDirectory.value;
        } catch (error) {
            console.log(">>> ERROR: Failed to build archive-based local vault", error);
            rootDirectory.value = null;
        }
    };

    const changeDirectory = (directory: Folder) => {
        try {
            if (directory?.paths) {
                return;
            }

            currentDirectory.value = directory;
        } catch (error) {
            console.log(">>> ERROR: Failed to change directory", error);
        }
    };

    // Generate a unique temp code for this load operation
    const localGenerateTempCode = () => {
        return generateTempCode('vault');
    };

    const getVaultStructure = async () => {
        console.log(">>> Getting vault structure with streaming...");

        // Generate new temp code for this load operation
        const tempCode = localGenerateTempCode();
        currentLoadCode.value = tempCode;
        console.log(">>> Generated temp code for vault load:", tempCode);

        // IMMEDIATELY clear all state to show loading
        vaultStructure.value = null;
        files.value = [];
        failedArchives.value = [];
        clearAllLoadingArchives();
        rootDirectory.value = null;
        currentDirectory.value = null;
        pendingVaultStructure.value = true;

        try {
            // Get vault key signature
            let vaultKeySignature = await walletStore.getVaultKeySignature();

            // Initialize vault structure 
            vaultStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };

            // Start streaming vault structure updates with temp code
            await invoke("get_vault_structure_streaming", {vaultKeySignature, tempCode});

        } catch (error: any) {
            console.log(">>> ERROR: Failed to get vault structure:", error);
            const message =
                error?.message || "There was an error getting your vault structure.";

            toast.add({
                severity: "error",
                summary: "Failed to get vault structure",
                detail: message,
                life: 3000,
            });

            // Reset loading state on error
            pendingVaultStructure.value = false;

            throw new Error("Failed to get vault structure");
        }
    };

    // Handle vault updates from streaming
    const handleVaultUpdate = (update: any) => {
        console.log(">>> Received vault update:", update.update_type, update);

        // Validate temp code - ignore update if it doesn't match current load operation
        if (!update.temp_code || update.temp_code !== currentLoadCode.value) {
            console.log(">>> Ignoring vault update - temp code mismatch:", update.temp_code, "vs", currentLoadCode.value);
            return;
        }

        if (!vaultStructure.value) {
            vaultStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };
        }

        switch (update.update_type) {
            case "IndividualFiles":
                // Add individual files immediately
                vaultStructure.value.files = update.files;

                // Update flattened files array
                update.files.forEach((file: IFileMetadata) => {
                    files.value.push({
                        path: file.path,
                        metadata: file.metadata,
                        file_access: file.file_type === "Private" ? {Private: null} : {Public: null},
                        is_loaded: file.is_loaded,
                        is_loading: false,
                        load_error: false
                    });
                });

                // Build initial directory structure with individual files
                buildRootDirectoryLegacy();

                // Hide loading once we have some content
                if (update.files.length > 0) {
                    pendingVaultStructure.value = false;
                }
                break;

            case "ArchiveLoading":
                if (update.loading_archive) {
                    addLoadingArchive(
                        update.loading_archive.name, 
                        update.loading_archive.address, 
                        update.loading_archive.is_private
                    );
                }
                break;

            case "ArchiveLoaded":
                if (update.archive) {
                    // Remove from loading list
                    removeLoadingArchive(update.archive.address, update.archive.is_private);

                    // Add the archive
                    const archive = {
                        name: update.archive.name,
                        archive_access: update.archive.is_private 
                            ? { Private: update.archive.address }
                            : { Public: update.archive.address },
                        files: update.archive.files
                    };
                    vaultStructure.value.archives.push(archive);

                    // Add archive files to flattened array
                    update.archive.files.forEach((file: IFileMetadata) => {
                        files.value.push({
                            path: file.path,
                            metadata: file.metadata,
                            file_access: file.file_type === "Private" ? {Private: null} : {Public: null},
                            is_loaded: false,
                            is_loading: false,
                            load_error: false
                        });
                    });

                    // Rebuild directory structure to include new archive
                    buildRootDirectoryLegacy();

                    // Hide loading once we have some content
                    if (files.value.length > 0) {
                        pendingVaultStructure.value = false;
                    }
                }
                break;

            case "ArchiveFailed":
                if (update.failed_archive) {
                    // Remove from loading list
                    removeLoadingArchive(update.failed_archive.address, update.failed_archive.is_private);

                    // Add to failed archives list, avoiding duplicates
                    const failedFileAccess = update.failed_archive.is_private 
                        ? { Private: update.failed_archive.address }
                        : { Public: update.failed_archive.address };
                    
                    const exists = failedArchives.value.some(a => {
                        const aAddress = 'Private' in a.archive_access ? a.archive_access.Private : a.archive_access.Public;
                        return aAddress === update.failed_archive.address;
                    });
                    if (!exists) {
                        const failedArchive = {
                            name: update.failed_archive.name,
                            archive_access: failedFileAccess
                        };
                        vaultStructure.value.failed_archives.push(failedArchive);
                        failedArchives.value.push(failedArchive);
                    }

                    // Rebuild directory to show failed archives
                    buildRootDirectoryLegacy();
                }
                break;

            case "Complete":
                // Clear any remaining loading archives and finish loading
                clearAllLoadingArchives();
                pendingVaultStructure.value = false;
                console.log(">>> Vault structure streaming completed");
                break;
        }
    };


    const getAllFiles = async () => {
        // This is now just an alias for the new two-phase approach
        return getVaultStructure();
    };

    const loadSingleFileData = async (file: any) => {
        try {
            // Mark the file as loading
            const fileIndex = files.value.findIndex(f => f.path === file.path);
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...files.value[fileIndex],
                    is_loading: true,
                    load_error: false
                };
            }

            // Get vault key signature
            let vaultKeySignature = await walletStore.getVaultKeySignature();

            // Load the file data
            const loadedFile = await invoke("get_single_file_data", {
                vaultKeySignature,
                filePath: file.path
            }) as any;

            // Update the file with loaded data
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...loadedFile,
                    is_loaded: true,
                    is_loading: false,
                    load_error: false
                };

                // Store in loadedFiles map
                loadedFiles.value.set(loadedFile.path, loadedFile);
            }

            return loadedFile;
        } catch (error: any) {
            console.error("Failed to load file data:", error);

            // Mark the file as failed to load
            const fileIndex = files.value.findIndex(f => f.path === file.path);
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...files.value[fileIndex],
                    is_loaded: false,
                    is_loading: false,
                    load_error: true
                };
            }

            throw error;
        }
    };

    // Return
    return {
        files,
        vaultStructure,
        failedArchives,
        loadingArchives,
        rootDirectory,
        currentDirectory,
        currentDirectoryFiles,
        pendingVaultStructure,
        loadedFiles,
        // Methods
        changeDirectory,
        getAllFiles,
        getVaultStructure,
        loadSingleFileData,
        handleVaultUpdate,
    };
});
