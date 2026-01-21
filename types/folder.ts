export interface IFolder {
  name: string;
  parent: IFolder;
  children: IFolder[];
}

export interface IArchive {
  name: string;
  archive_access: {
    Private: string;
  } | {
    Public: string;
  };
  files: IFileMetadata[];
}

export interface IVaultStructure {
  archives: IArchive[];
  failed_archives: IFailedArchive[];
  files: IFileMetadata[];
}

export interface IFailedArchive {
  name: string;
  archive_access: {
    Private: string;
  } | {
    Public: string;
  };
}

export interface IFileMetadata {
  path: string;
  metadata: {
    uploaded: number;
    created: number;
    modified: number;
    size: number;
  };
  file_type: 'Public' | 'Private';
  is_loaded: boolean;
  archive_name: string;
  access_data?: {
    Public?: string; // Data address string
    Private?: any; // DataMapChunk bytes
  };
}

export interface IFile {
  file_access: {
    Private: any; // Bytes array containing datamap chunk
  } | {
    Public: string; // Data address string
  }
  metadata: {
    uploaded: number;
    created: number;
    modified: number;
    size: number;
  }
  path: string;
  is_loaded?: boolean;
  is_loading?: boolean;
  load_error?: boolean;
}

/**
 * Upload progress step status
 */
export type UploadStepStatus = 'pending' | 'processing' | 'completed' | 'error';

/**
 * Upload progress step for tracking multi-stage uploads
 */
export interface UploadStep {
  key: string;
  label: string;
  status: UploadStepStatus;
  message?: string;
  progress?: number;
}

/**
 * Quote data received from the network for an upload
 */
export interface QuoteData {
  totalFiles: number;
  totalSize: string;
  totalCostFormatted?: string;
  pricePerMB?: string;
  paymentRequired?: boolean;
  paymentOrderId?: string;
  totalCostNano?: string;
  costPerFileNano?: string;
  payments?: PaymentInfo[];
  rawPayments?: [string, string, string][];  // [quoteHash, rewardsAddress, amount]
  rawQuoteData?: unknown;
}

/**
 * Payment info for a single quote payment
 */
export interface PaymentInfo {
  quoteHash: string;
  rewardsAddress: string;
  amount: string;
}

/**
 * Options for file upload
 */
export interface UploadOptionsData {
  makePublic: boolean;
  existingArchiveAddress?: string;
}

/**
 * File type for access control
 */
export type FileAccessType = 'public_file' | 'private_file';

/**
 * Archive access structure for private or public archives
 */
export type ArchiveAccess = { Private: string } | { Public: string };

/**
 * File access structure - Private contains bytes, Public contains address string
 */
export type FileAccessData = { Private: number[] } | { Public: string };
