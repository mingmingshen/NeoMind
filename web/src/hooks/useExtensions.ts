/**
 * Hook for managing dynamic extensions (.so/.wasm).
 *
 * Extensions are dynamically loaded code modules that extend NeoTalk's capabilities.
 * They are distinct from user configurations like LLM backends or device connections.
 */

import { useState, useCallback, useEffect } from 'react';
import { fetchAPI } from '@/lib/api';

/** Extension data returned from API */
export interface Extension {
  id: string;
  name: string;
  extension_type: string;
  version: string;
  description?: string;
  author?: string;
  state: string;
  file_path?: string;
  loaded_at?: number;
}

/** Extension statistics */
export interface ExtensionStats {
  start_count: number;
  stop_count: number;
  error_count: number;
  last_error?: string;
}

/** Extension type definition */
export interface ExtensionType {
  id: string;
  name: string;
  description: string;
}

/** Query parameters for listing extensions */
export interface ListExtensionsParams {
  extension_type?: string;
  state?: string;
}

/** Request to register an extension */
export interface RegisterExtensionRequest {
  file_path: string;
  auto_start?: boolean;
}

/** Request to execute an extension command */
export interface ExecuteCommandRequest {
  command: string;
  args?: Record<string, unknown>;
}

/**
 * Hook for managing extensions.
 */
export function useExtensions() {
  const [extensions, setExtensions] = useState<Extension[]>([]);
  const [extensionTypes, setExtensionTypes] = useState<ExtensionType[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Fetch all extensions */
  const fetchExtensions = useCallback(async (params?: ListExtensionsParams) => {
    setLoading(true);
    setError(null);
    try {
      const queryParams = new URLSearchParams();
      if (params?.extension_type) {
        queryParams.set('extension_type', params.extension_type);
      }
      if (params?.state) {
        queryParams.set('state', params.state);
      }
      const query = queryParams.toString();
      const url = query ? `/api/extensions?${query}` : '/api/extensions';
      
      const response = await fetchAPI<{ data: Extension[] }>(url);
      if (response?.data) {
        setExtensions(response.data);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch extensions');
    } finally {
      setLoading(false);
    }
  }, []);

  /** Fetch extension types */
  const fetchExtensionTypes = useCallback(async () => {
    try {
      const response = await fetchAPI<{ data: ExtensionType[] }>('/api/extensions/types');
      if (response?.data) {
        setExtensionTypes(response.data);
      }
    } catch (err) {
      console.error('Failed to fetch extension types:', err);
    }
  }, []);

  /** Get a single extension */
  const getExtension = useCallback(async (id: string): Promise<Extension | null> => {
    try {
      const response = await fetchAPI<{ data: Extension }>(`/api/extensions/${id}`);
      return response?.data || null;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get extension');
      return null;
    }
  }, []);

  /** Get extension statistics */
  const getExtensionStats = useCallback(async (id: string): Promise<ExtensionStats | null> => {
    try {
      const response = await fetchAPI<{ data: ExtensionStats }>(`/api/extensions/${id}/stats`);
      return response?.data || null;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get extension stats');
      return null;
    }
  }, []);

  /** Discover extensions in configured directories */
  const discoverExtensions = useCallback(async () => {
    setLoading(true);
    try {
      const response = await fetchAPI<{ data: unknown[] }>('/api/extensions/discover', {
        method: 'POST',
      });
      return response?.data || [];
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to discover extensions');
      return [];
    } finally {
      setLoading(false);
    }
  }, []);

  /** Register an extension */
  const registerExtension = useCallback(async (request: RegisterExtensionRequest) => {
    setLoading(true);
    try {
      const response = await fetchAPI<{ data: unknown }>('/api/extensions', {
        method: 'POST',
        body: JSON.stringify(request),
      });
      await fetchExtensions();
      return response?.data;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to register extension');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [fetchExtensions]);

  /** Unregister an extension */
  const unregisterExtension = useCallback(async (id: string) => {
    setLoading(true);
    try {
      await fetchAPI(`/api/extensions/${id}`, {
        method: 'DELETE',
      });
      await fetchExtensions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to unregister extension');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [fetchExtensions]);

  /** Start an extension */
  const startExtension = useCallback(async (id: string) => {
    try {
      await fetchAPI(`/api/extensions/${id}/start`, {
        method: 'POST',
      });
      await fetchExtensions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start extension');
      throw err;
    }
  }, [fetchExtensions]);

  /** Stop an extension */
  const stopExtension = useCallback(async (id: string) => {
    try {
      await fetchAPI(`/api/extensions/${id}/stop`, {
        method: 'POST',
      });
      await fetchExtensions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop extension');
      throw err;
    }
  }, [fetchExtensions]);

  /** Check extension health */
  const checkHealth = useCallback(async (id: string): Promise<boolean> => {
    try {
      const response = await fetchAPI<{ data: { healthy: boolean } }>(`/api/extensions/${id}/health`);
      return response?.data?.healthy ?? false;
    } catch {
      return false;
    }
  }, []);

  /** Execute a command on an extension */
  const executeCommand = useCallback(async (id: string, request: ExecuteCommandRequest) => {
    try {
      const response = await fetchAPI<{ data: unknown }>(`/api/extensions/${id}/command`, {
        method: 'POST',
        body: JSON.stringify(request),
      });
      return response?.data;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to execute command');
      throw err;
    }
  }, []);

  /** Clear error */
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Fetch extensions and types on mount
  useEffect(() => {
    fetchExtensions();
    fetchExtensionTypes();
  }, [fetchExtensions, fetchExtensionTypes]);

  return {
    // State
    extensions,
    extensionTypes,
    loading,
    error,

    // Actions
    fetchExtensions,
    fetchExtensionTypes,
    getExtension,
    getExtensionStats,
    discoverExtensions,
    registerExtension,
    unregisterExtension,
    startExtension,
    stopExtension,
    checkHealth,
    executeCommand,
    clearError,
  };
}

export default useExtensions;
