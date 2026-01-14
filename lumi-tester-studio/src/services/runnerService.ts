import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { StepResult, TestResult } from '../types';
import { useExecutionStateStore, useFileStore } from '../stores';
import { findFileById } from '../utils/treeUtils';

export const runTestFlow = async (
    yamlContent: string,
    fileId: string,
    fileName: string,
    platform: string,
    device: string | null,
    onUpdate: (result: Partial<TestResult>) => void,
    signal: AbortSignal
): Promise<TestResult> => {
    // signal is used if we implement cancellation via backend, 
    // but for now we just pass it to satisfy interface or could remove if unused.
    // If unused by invoke, just suppressing or using it?
    // Invoke doesn't support signal directly unless we manually call cancel.
    // Let's pretend to use it or suppress warning.
    void signal;

    return new Promise(async (resolve) => {
        // Clear previous state for this file
        const stateStore = useExecutionStateStore.getState();
        stateStore.clearFileState(fileId);
        // Start new execution state
        stateStore.startFileExecution(fileId, fileName);
        // Map steps to lines immediately from yamlContent
        stateStore.mapStepsFromContent(fileId, yamlContent);
        
        console.log('[Runner] Started execution for file:', {
            fileId,
            fileName,
            yamlContentLength: yamlContent.length
        });

        const startTime = Date.now();
        const steps: StepResult[] = [];
        let currentResult: TestResult = {
            id: `run-${Date.now()}`,
            fileId: fileId,
            fileName: fileName,
            status: 'running',
            timestamp: startTime,
            totalDuration: 0,
            passed: 0,
            failed: 0,
            steps: []
        };

        // Helper to update result
        const update = () => {
            const passed = steps.filter(s => s.status === 'passed').length;
            const failed = steps.filter(s => s.status === 'failed').length;
            currentResult = {
                ...currentResult,
                steps: [...steps],
                totalDuration: Date.now() - startTime,
                passed,
                failed
            };
            onUpdate(currentResult);
        };

        update();

        let unlisten: UnlistenFn | undefined;

        // Track global offset for steps and flow path stack
        let globalOffset = 0;
        // Stack to track [path, offset] pairs - we need to restore the offset that was active before sub-flow started
        const flowPathStack: Array<{ path: string; offset: number }> = [{ path: fileId, offset: 0 }];
        let currentFlowPath = fileId;
        // Track the runFlow step index in parent file when sub-flow starts
        let parentRunFlowStepIndex: number | null = null;
        
        // Get base directory from original fileId for resolving relative paths
        const getBaseDir = (filePath: string): string => {
            const pathParts = filePath.split('/');
            if (pathParts.length > 1) {
                pathParts.pop(); // Remove filename
                return pathParts.join('/');
            }
            return filePath;
        };
        const baseDir = getBaseDir(fileId);

        // Subscribe to events from Rust
        try {
            unlisten = await listen<any>('test-event', (event) => {
                const payload = event.payload;
                console.log('[Lumi Event]', payload);

                if (payload.type === 'FlowStarted') {
                    // Save current offset before starting new flow
                    const currentOffset = globalOffset;
                    // Update offset for NEW commands (this happens before commands are added)
                    globalOffset = steps.length;

                    // Determine the flow path for this flow
                    let flowPathForThisFlow: string;
                    if (payload.depth === 0) {
                        // For main flow (depth = 0), use the original fileId if flow_path is temp path
                        if (payload.flow_path && payload.flow_path.includes('.lumi_tmp_run')) {
                            flowPathForThisFlow = fileId;
                        } else if (payload.flow_path) {
                            flowPathForThisFlow = payload.flow_path;
                        } else {
                            flowPathForThisFlow = fileId;
                        }
                        // Update currentFlowPath for main flow
                        currentFlowPath = flowPathForThisFlow;
                    } else {
                        // For sub-flow (depth > 0), use the flow_path from payload
                        flowPathForThisFlow = payload.flow_path || currentFlowPath;
                        // Don't update currentFlowPath yet - we'll update it after processing
                    }
                    
                    // Push current path and offset before starting sub-flow
                    // For depth = 0, push the main file path
                    // For depth > 0, push the parent's currentFlowPath
                    const pathToPush = payload.depth === 0 ? currentFlowPath : currentFlowPath;
                    flowPathStack.push({ path: pathToPush, offset: currentOffset });
                    
                    // For sub-flow, update currentFlowPath after pushing parent to stack
                    if (payload.depth > 0) {
                        currentFlowPath = flowPathForThisFlow;
                    }
                    
                    const stateStore = useExecutionStateStore.getState();
                    const fileStore = useFileStore.getState();
                    
                    // Helper to find fileId from flow_path (handles both absolute and relative paths)
                    const findFileIdFromPath = (path: string): string => {
                        // Normalize path (remove .lumi_tmp_run if present)
                        let normalizedPath = path;
                        if (path.includes('.lumi_tmp_run')) {
                            normalizedPath = path.replace(/\.lumi_tmp_run$/, '');
                        }
                        
                        // If path is absolute, try exact match first
                        const files = fileStore.files;
                        const exactMatch = findFileById(files, normalizedPath);
                        if (exactMatch) return exactMatch.id;
                        
                        // If path is relative, resolve it against baseDir
                        if (!normalizedPath.startsWith('/') && !normalizedPath.match(/^[A-Za-z]:/)) {
                            // Relative path - resolve against baseDir
                            const resolvedPath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
                            const resolvedMatch = findFileById(files, resolvedPath);
                            if (resolvedMatch) return resolvedMatch.id;
                            
                            // Also try with current flow's directory
                            const currentBaseDir = getBaseDir(currentFlowPath);
                            if (currentBaseDir !== baseDir) {
                                const altResolvedPath = `${currentBaseDir}/${normalizedPath}`;
                                const altMatch = findFileById(files, altResolvedPath);
                                if (altMatch) return altMatch.id;
                            }
                        }
                        
                        // Try to find by filename (last part of path)
                        const fileName = normalizedPath.split('/').pop() || normalizedPath.split('\\').pop() || '';
                        if (fileName) {
                            // Search in file tree by name
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.name === fileName && node.type === 'file') {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                            
                            // Try matching end of path
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.id.endsWith(normalizedPath) || node.id.endsWith(`/${normalizedPath}`)) {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                        }
                        
                        // Fallback: return normalized path (will be used as-is if not found)
                        console.warn('[Runner] findFileIdFromPath: Could not find file for path:', normalizedPath, 'returning as-is');
                        return normalizedPath;
                    };
                    
                    // Find actualFileId based on flow type
                    let actualFileId: string;
                    if (payload.depth === 0) {
                        // For main flow, use currentFlowPath (which is already set to fileId or normalized path)
                        actualFileId = findFileIdFromPath(currentFlowPath) || currentFlowPath;
                    } else {
                        // For sub-flow, use flowPathForThisFlow (from payload.flow_path)
                        actualFileId = findFileIdFromPath(flowPathForThisFlow) || flowPathForThisFlow;
                    }
                    
                    // If this is the main flow (depth = 0), ensure execution state is started
                    if (payload.depth === 0) {
                        // This is the main file being run - ensure execution state exists
                        let fileState = stateStore.getFileState(actualFileId);
                        if (!fileState) {
                            const mainFileName = actualFileId.split('/').pop() || actualFileId.split('\\').pop() || payload.flow_name;
                            stateStore.startFileExecution(actualFileId, mainFileName);
                            fileState = stateStore.getFileState(actualFileId);
                        }
                        
                        // Try to map steps if not already mapped
                        if (fileState && fileState.stepLines.size === 0) {
                            const mainFile = findFileById(fileStore.files, actualFileId);
                            if (mainFile && mainFile.content) {
                                console.log('[Runner] FlowStarted (main flow): Mapping steps for', actualFileId);
                                stateStore.mapStepsFromContent(actualFileId, mainFile.content);
                            }
                        }
                    }
                    // If this is a sub-flow (depth > 0), track it and highlight runFlow in parent
                    else if (payload.depth > 0) {
                        // Start execution state for sub-flow file
                        const subFlowFileName = actualFileId.split('/').pop() || actualFileId.split('\\').pop() || payload.flow_name;
                        stateStore.startFileExecution(actualFileId, subFlowFileName);
                        
                        // Try to load content and map steps for sub-flow
                        const subFlowFile = findFileById(fileStore.files, actualFileId);
                        if (subFlowFile && subFlowFile.content) {
                            stateStore.mapStepsFromContent(actualFileId, subFlowFile.content);
                        }
                        
                        // Find and highlight the runFlow step in parent file
                        // Use the offset that was saved before this sub-flow started (currentOffset)
                        // currentOffset is the number of steps that have been executed in parent file,
                        // which is exactly the index of the runFlow step that triggered this sub-flow
                        const fileState = stateStore.getFileState(fileId);
                        if (fileState && currentOffset >= 0) {
                            // The runFlow step is at the offset that was active before sub-flow started
                            const runFlowStepIndex = currentOffset;
                            parentRunFlowStepIndex = runFlowStepIndex;
                            
                            console.log('[Runner] FlowStarted (sub-flow): Setting parentRunFlowStepIndex to', runFlowStepIndex, 'for parent file', fileId);
                            
                            const lineNumber = fileState.stepLines.get(runFlowStepIndex);
                            if (lineNumber !== undefined) {
                                stateStore.setExecutingStep(fileId, runFlowStepIndex, lineNumber);
                                stateStore.setStepStatus(fileId, runFlowStepIndex, 'running');
                                console.log('[Runner] FlowStarted (sub-flow): Highlighting runFlow step', runFlowStepIndex, 'at line', lineNumber, 'in parent file');
                            } else {
                                console.warn('[Runner] FlowStarted (sub-flow): Could not find line number for runFlow step', runFlowStepIndex, 'Available step lines:', Array.from(fileState.stepLines.entries()));
                            }
                        } else {
                            console.warn('[Runner] FlowStarted (sub-flow): Could not find parent file state or currentOffset is invalid', {
                                fileId,
                                currentOffset,
                                hasFileState: !!fileState
                            });
                        }
                    }
                    
                    console.log('[Runner] FlowStarted:', payload.flow_name, 'path:', currentFlowPath, 'actualFileId:', actualFileId, 'depth:', payload.depth);
                }
                else if (payload.type === 'FlowFinished') {
                    const stateStore = useExecutionStateStore.getState();
                    const fileStore = useFileStore.getState();
                    
                    // Helper to find fileId from flow_path (reuse same logic)
                    const findFileIdFromPath = (path: string): string => {
                        let normalizedPath = path;
                        if (path.includes('.lumi_tmp_run')) {
                            normalizedPath = path.replace(/\.lumi_tmp_run$/, '');
                        }
                        
                        // If path is absolute, try exact match first
                        const files = fileStore.files;
                        const exactMatch = findFileById(files, normalizedPath);
                        if (exactMatch) return exactMatch.id;
                        
                        // If path is relative, resolve it against baseDir
                        if (!normalizedPath.startsWith('/') && !normalizedPath.match(/^[A-Za-z]:/)) {
                            const resolvedPath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
                            const resolvedMatch = findFileById(files, resolvedPath);
                            if (resolvedMatch) return resolvedMatch.id;
                            
                            const currentBaseDir = getBaseDir(currentFlowPath);
                            if (currentBaseDir !== baseDir) {
                                const altResolvedPath = `${currentBaseDir}/${normalizedPath}`;
                                const altMatch = findFileById(files, altResolvedPath);
                                if (altMatch) return altMatch.id;
                            }
                        }
                        
                        // Try by filename
                        const fileName = normalizedPath.split('/').pop() || normalizedPath.split('\\').pop() || '';
                        if (fileName) {
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.name === fileName && node.type === 'file') {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                        }
                        
                        return normalizedPath;
                    };
                    
                    // Find finishedFileId BEFORE restoring currentFlowPath (while currentFlowPath still points to finished flow)
                    const finishedFileId = findFileIdFromPath(currentFlowPath);
                    console.log('[Runner] FlowFinished: Finished flow path:', currentFlowPath, 'resolved to:', finishedFileId);
                    
                    // If this was a sub-flow finishing, mark the runFlow step as passed/failed in parent
                    if (payload.depth > 0 && parentRunFlowStepIndex !== null) {
                        const status = payload.status === 'Passed' ? 'passed' : 'failed';
                        console.log('[Runner] FlowFinished (sub-flow): Marking runFlow step', parentRunFlowStepIndex, 'as', status, 'in parent file', fileId, 'depth:', payload.depth);
                        
                        // Get the parent file state to check if runFlow step exists
                        const parentFileState = stateStore.getFileState(fileId);
                        if (parentFileState) {
                            // Set status for runFlow step
                            stateStore.setStepStatus(fileId, parentRunFlowStepIndex, status);
                            // Clear executing step for parent file's runFlow step
                            stateStore.clearExecutingStep(fileId);
                            console.log('[Runner] FlowFinished (sub-flow): Cleared executing step for parent file, runFlow step', parentRunFlowStepIndex, 'marked as', status);
                        } else {
                            console.warn('[Runner] FlowFinished (sub-flow): Parent file state not found for', fileId);
                        }
                        parentRunFlowStepIndex = null;
                    } else if (payload.depth > 0) {
                        console.warn('[Runner] FlowFinished (sub-flow): parentRunFlowStepIndex is null, cannot mark runFlow step in parent file');
                    }
                    
                    // Stop execution state for the finished flow and clear executing step
                    console.log('[Runner] FlowFinished: Clearing executing step for finished flow', finishedFileId);
                    const finishedFileState = stateStore.getFileState(finishedFileId);
                    if (finishedFileState) {
                        console.log('[Runner] FlowFinished: Finished file state before clear:', {
                            executingStepIndex: finishedFileState.executingStepIndex,
                            executingLine: finishedFileState.executingLine,
                            stepStatusesSize: finishedFileState.stepStatuses.size
                        });
                        // Clear executing step for the finished flow
                        stateStore.clearExecutingStep(finishedFileId);
                        // Stop execution
                        stateStore.stopFileExecution(finishedFileId);
                        console.log('[Runner] FlowFinished: Cleared and stopped execution for', finishedFileId);
                    } else {
                        console.warn('[Runner] FlowFinished: File state not found for', finishedFileId);
                    }
                    
                    // Pop path and restore offset to the value before this flow started
                    const popped = flowPathStack.pop();
                    if (popped) {
                        // Restore globalOffset to the value that was active before this flow started
                        globalOffset = popped.offset;
                        // Normalize restored path to actual fileId (not temp path)
                        const normalizedRestoredPath = findFileIdFromPath(popped.path);
                        currentFlowPath = normalizedRestoredPath || popped.path;
                        console.log('[Runner] FlowFinished. Restored path:', popped.path, 'normalized to:', currentFlowPath, 'restored offset:', globalOffset);
                    } else {
                        // Fallback if stack is empty
                        currentFlowPath = fileId;
                        globalOffset = 0;
                        console.warn('[Runner] FlowFinished: Stack empty, resetting to fileId and offset 0');
                    }
                }
                else if (payload.type === 'CommandStarted') {
                    const absIndex = globalOffset + payload.index;

                    const stepData = {
                        name: payload.command,
                        status: 'running' as const, // Fix type inference
                        logs: [] as string[],
                        timestamp: Date.now(),
                        fileId: currentFlowPath,
                        localIndex: payload.index
                    };

                    if (steps[absIndex]) {
                        Object.assign(steps[absIndex], stepData);
                    } else {
                        steps[absIndex] = stepData;
                    }

                    // Update execution state store
                    const stateStore = useExecutionStateStore.getState();
                    const fileStore = useFileStore.getState();
                    
                    // Helper to find fileId from flow_path (reuse same logic as FlowStarted)
                    const findFileIdFromPath = (path: string): string => {
                        let normalizedPath = path;
                        if (path.includes('.lumi_tmp_run')) {
                            normalizedPath = path.replace(/\.lumi_tmp_run$/, '');
                        }
                        
                        // If path is absolute, try exact match first
                        const files = fileStore.files;
                        const exactMatch = findFileById(files, normalizedPath);
                        if (exactMatch) return exactMatch.id;
                        
                        // If path is relative, resolve it against baseDir
                        if (!normalizedPath.startsWith('/') && !normalizedPath.match(/^[A-Za-z]:/)) {
                            const resolvedPath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
                            const resolvedMatch = findFileById(files, resolvedPath);
                            if (resolvedMatch) return resolvedMatch.id;
                            
                            const currentBaseDir = getBaseDir(currentFlowPath);
                            if (currentBaseDir !== baseDir) {
                                const altResolvedPath = `${currentBaseDir}/${normalizedPath}`;
                                const altMatch = findFileById(files, altResolvedPath);
                                if (altMatch) return altMatch.id;
                            }
                        }
                        
                        // Try by filename
                        const fileName = normalizedPath.split('/').pop() || normalizedPath.split('\\').pop() || '';
                        if (fileName) {
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.name === fileName && node.type === 'file') {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                        }
                        
                        return normalizedPath;
                    };
                    
                    const targetFileId = findFileIdFromPath(currentFlowPath);
                    
                    // Update execution state for the file that contains this command
                    let fileState = stateStore.getFileState(targetFileId);
                    
                    // If file state doesn't exist, create it
                    if (!fileState) {
                        const fileName = targetFileId.split('/').pop() || targetFileId.split('\\').pop() || 'unknown';
                        stateStore.startFileExecution(targetFileId, fileName);
                        fileState = stateStore.getFileState(targetFileId);
                    }
                    
                    // If stepLines are not mapped yet, try to map them
                    if (fileState && fileState.stepLines.size === 0) {
                        const targetFile = findFileById(fileStore.files, targetFileId);
                        if (targetFile && targetFile.content) {
                            console.log('[Runner] CommandStarted: Mapping steps for', targetFileId);
                            stateStore.mapStepsFromContent(targetFileId, targetFile.content);
                            fileState = stateStore.getFileState(targetFileId);
                        } else if (targetFile && !targetFile.content) {
                            // Try to load content
                            const loadContent = fileStore.loadContent;
                            if (loadContent) {
                                loadContent(targetFileId).then(() => {
                                    const updatedFile = findFileById(fileStore.files, targetFileId);
                                    if (updatedFile && updatedFile.content) {
                                        stateStore.mapStepsFromContent(targetFileId, updatedFile.content);
                                    }
                                });
                            }
                        }
                    }
                    
                    console.log('[Runner] CommandStarted:', {
                        targetFileId,
                        stepIndex: payload.index,
                        command: payload.command,
                        fileStateExists: !!fileState,
                        stepLinesSize: fileState?.stepLines.size ?? 0,
                        currentFlowPath,
                        depth: payload.depth
                    });
                    
                    if (fileState) {
                        stateStore.setStepStatus(targetFileId, payload.index, 'running');
                        
                        // Re-check stepLines after potential mapping
                        const updatedFileState = stateStore.getFileState(targetFileId);
                        const lineNumber = updatedFileState?.stepLines.get(payload.index);
                        
                        if (lineNumber !== undefined && lineNumber >= 0) {
                            console.log('[Runner] Found line number from store:', lineNumber, 'for step index:', payload.index);
                            stateStore.setExecutingStep(targetFileId, payload.index, lineNumber);
                        } else {
                            console.log('[Runner] Line number not found in store for step', payload.index, 'stepLines size:', updatedFileState?.stepLines.size ?? 0, 'will be updated by Editor');
                            // Set executing step - line number will be updated by Editor when it maps steps
                            stateStore.setExecutingStep(targetFileId, payload.index, -1);
                        }
                    } else {
                        console.warn('[Runner] CommandStarted: File state not found for', targetFileId);
                    }

                    update();
                }
                // ... (CommandPassed/Failed/Log handlers need minimal updates or none if they use absIndex logic which is fine)
                else if (payload.type === 'CommandPassed') {
                    const absIndex = globalOffset + payload.index;
                    if (steps[absIndex]) {
                        steps[absIndex].status = 'passed';
                    }

                    // Update execution state store for the file containing this command
                    const stateStore = useExecutionStateStore.getState();
                    const fileStore = useFileStore.getState();
                    
                    // Helper to find fileId from flow_path (reuse same logic)
                    const findFileIdFromPath = (path: string): string => {
                        let normalizedPath = path;
                        if (path.includes('.lumi_tmp_run')) {
                            normalizedPath = path.replace(/\.lumi_tmp_run$/, '');
                        }
                        
                        // If path is absolute, try exact match first
                        const files = fileStore.files;
                        const exactMatch = findFileById(files, normalizedPath);
                        if (exactMatch) return exactMatch.id;
                        
                        // If path is relative, resolve it against baseDir
                        if (!normalizedPath.startsWith('/') && !normalizedPath.match(/^[A-Za-z]:/)) {
                            const resolvedPath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
                            const resolvedMatch = findFileById(files, resolvedPath);
                            if (resolvedMatch) return resolvedMatch.id;
                            
                            const currentBaseDir = getBaseDir(currentFlowPath);
                            if (currentBaseDir !== baseDir) {
                                const altResolvedPath = `${currentBaseDir}/${normalizedPath}`;
                                const altMatch = findFileById(files, altResolvedPath);
                                if (altMatch) return altMatch.id;
                            }
                        }
                        
                        // Try by filename
                        const fileName = normalizedPath.split('/').pop() || normalizedPath.split('\\').pop() || '';
                        if (fileName) {
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.name === fileName && node.type === 'file') {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                        }
                        
                        return normalizedPath;
                    };
                    const targetFileId = findFileIdFromPath(currentFlowPath);
                    
                    console.log('[Runner] CommandPassed: Setting status for', targetFileId, 'step', payload.index);
                    stateStore.setStepStatus(targetFileId, payload.index, 'passed');
                    stateStore.clearExecutingStep(targetFileId);
                    
                    console.log('[Runner] CommandPassed:', {
                        targetFileId,
                        stepIndex: payload.index
                    });

                    update();
                }
                else if (payload.type === 'CommandFailed') {
                    const absIndex = globalOffset + payload.index;
                    if (steps[absIndex]) {
                        steps[absIndex].status = 'failed';
                        steps[absIndex].error = payload.error;
                        steps[absIndex].logs?.push(`Error: ${payload.error}`);
                    }

                    // Update execution state store for the file containing this command
                    const stateStore = useExecutionStateStore.getState();
                    const fileStore = useFileStore.getState();
                    
                    // Helper to find fileId from flow_path (reuse same logic)
                    const findFileIdFromPath = (path: string): string => {
                        let normalizedPath = path;
                        if (path.includes('.lumi_tmp_run')) {
                            normalizedPath = path.replace(/\.lumi_tmp_run$/, '');
                        }
                        
                        // If path is absolute, try exact match first
                        const files = fileStore.files;
                        const exactMatch = findFileById(files, normalizedPath);
                        if (exactMatch) return exactMatch.id;
                        
                        // If path is relative, resolve it against baseDir
                        if (!normalizedPath.startsWith('/') && !normalizedPath.match(/^[A-Za-z]:/)) {
                            const resolvedPath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
                            const resolvedMatch = findFileById(files, resolvedPath);
                            if (resolvedMatch) return resolvedMatch.id;
                            
                            const currentBaseDir = getBaseDir(currentFlowPath);
                            if (currentBaseDir !== baseDir) {
                                const altResolvedPath = `${currentBaseDir}/${normalizedPath}`;
                                const altMatch = findFileById(files, altResolvedPath);
                                if (altMatch) return altMatch.id;
                            }
                        }
                        
                        // Try by filename
                        const fileName = normalizedPath.split('/').pop() || normalizedPath.split('\\').pop() || '';
                        if (fileName) {
                            for (const file of files) {
                                const searchInTree = (node: any): any => {
                                    if (node.name === fileName && node.type === 'file') {
                                        return node.id;
                                    }
                                    if (node.children) {
                                        for (const child of node.children) {
                                            const result = searchInTree(child);
                                            if (result) return result;
                                        }
                                    }
                                    return null;
                                };
                                const result = searchInTree(file);
                                if (result) return result;
                            }
                        }
                        
                        return normalizedPath;
                    };
                    const targetFileId = findFileIdFromPath(currentFlowPath);
                    
                    console.log('[Runner] CommandFailed: Setting status for', targetFileId, 'step', payload.index);
                    stateStore.setStepStatus(targetFileId, payload.index, 'failed');
                    if (payload.error) {
                        stateStore.setStepError(targetFileId, payload.index, payload.error);
                    }
                    stateStore.clearExecutingStep(targetFileId);
                    
                    console.log('[Runner] CommandFailed:', {
                        targetFileId,
                        stepIndex: payload.index,
                        error: payload.error
                    });

                    update();
                }
                else if (payload.type === 'Log') {
                    const runningStep = [...steps].reverse().find(s => s.status === 'running');
                    if (runningStep) {
                        runningStep.logs?.push(payload.message);
                    } else if (steps.length > 0) {
                        steps[steps.length - 1].logs?.push(payload.message);
                    }
                    update();
                }
            });

            // Start the run
            // Defaulting platform to 'android' for now. 
            // TODO: Expose platform selection in UI.
            await invoke('run_test_flow', {
                content: yamlContent,
                filename: fileName,
                filePath: fileId,
                platform: platform,
                device: device
            });

            // Run finished successfully (if invoke returns)
            currentResult.status = 'passed'; // Or determined by steps?
            if (steps.some(s => s.status === 'failed')) {
                currentResult.status = 'failed';
            }
            update();

            // Stop file execution state
            useExecutionStateStore.getState().stopFileExecution(fileId);

            resolve(currentResult);

        } catch (error) {
            console.error('Run failed', error);
            currentResult.status = 'failed';
            // currentResult.error = String(error);
            update();

            // Stop file execution state
            useExecutionStateStore.getState().stopFileExecution(fileId);

            resolve(currentResult);
        } finally {
            if (unlisten) unlisten();
        }
    });
};
