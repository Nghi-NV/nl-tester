import { FileNode } from '../types';

/**
 * Find a node in the tree by ID
 */
export const findFileById = (nodes: FileNode[], id: string | null): FileNode | null => {
  if (!id) return null;

  for (const node of nodes) {
    if (node.id === id) return node;
    if (node.children) {
      const found = findFileById(node.children, id);
      if (found) return found;
    }
  }
  return null;
};

/**
 * Find a file by name in the tree
 */
export const findFileByName = (nodes: FileNode[], name: string): FileNode | null => {
  for (const node of nodes) {
    if (node.type === 'file' && node.name === name) return node;
    if (node.children) {
      const found = findFileByName(node.children, name);
      if (found) return found;
    }
  }
  return null;
};

/**
 * Get all descendant files from a node
 */
export const getAllDescendantFiles = (node: FileNode): FileNode[] => {
  if (node.type === 'file') return [node];

  let results: FileNode[] = [];
  if (node.children) {
    for (const child of node.children) {
      results = [...results, ...getAllDescendantFiles(child)];
    }
  }
  return results;
};

/**
 * Map over tree nodes with a transformation function
 */
export const mapTree = (
  nodes: FileNode[],
  mapper: (node: FileNode) => FileNode
): FileNode[] => {
  return nodes.map(node => {
    const mapped = mapper(node);
    if (mapped.children) {
      return { ...mapped, children: mapTree(mapped.children, mapper) };
    }
    return mapped;
  });
};

/**
 * Filter tree nodes (removes nodes that don't match, keeps children of kept nodes)
 */
export const filterTree = (
  nodes: FileNode[],
  predicate: (node: FileNode) => boolean
): FileNode[] => {
  return nodes
    .filter(predicate)
    .map(node => ({
      ...node,
      children: node.children ? filterTree(node.children, predicate) : undefined,
    }));
};

/**
 * Add a node to the tree at a specific parent
 */
export const addNodeToTree = (
  nodes: FileNode[],
  parentId: string | null,
  newNode: FileNode
): FileNode[] => {
  if (!parentId) {
    return [...nodes, newNode];
  }

  return mapTree(nodes, node => {
    if (node.id === parentId && node.type === 'folder') {
      return {
        ...node,
        children: [...(node.children || []), newNode],
        isOpen: true,
      };
    }
    return node;
  });
};

/**
 * Update a node in the tree by ID
 */
export const updateNodeInTree = (
  nodes: FileNode[],
  id: string,
  updater: (node: FileNode) => FileNode
): FileNode[] => {
  return mapTree(nodes, node => {
    if (node.id === id) {
      return updater(node);
    }
    return node;
  });
};

/**
 * Delete a node from the tree by ID
 */
export const deleteNodeFromTree = (
  nodes: FileNode[],
  id: string
): FileNode[] => {
  return filterTree(nodes, node => node.id !== id);
};

/**
 * Toggle folder open/closed state
 */
export const toggleFolderInTree = (
  nodes: FileNode[],
  id: string
): FileNode[] => {
  return updateNodeInTree(nodes, id, node => ({
    ...node,
    isOpen: !node.isOpen,
  }));
};

/**
 * Update file content in tree
 */
export const updateFileContentInTree = (
  nodes: FileNode[],
  id: string,
  content: string
): FileNode[] => {
  return updateNodeInTree(nodes, id, node => ({
    ...node,
    content,
  }));
};

/**
 * Rename a node in the tree
 */
export const renameNodeInTree = (
  nodes: FileNode[],
  id: string,
  newName: string
): FileNode[] => {
  return updateNodeInTree(nodes, id, node => ({
    ...node,
    name: newName,
  }));
};

/**
 * Move a node in the tree
 */
export const moveNodeInTree = (
  nodes: FileNode[],
  sourceId: string,
  targetParentId: string | null,
  index: number
): FileNode[] => {
  // 1. Find the node
  const nodeToMove = findFileById(nodes, sourceId);
  if (!nodeToMove) return nodes;

  // 2. Remove from old location
  const nodesWithoutSource = deleteNodeFromTree(nodes, sourceId);

  // 3. Insert at new location
  if (!targetParentId) {
    const newNodes = [...nodesWithoutSource];
    // Ensure index is within bounds
    const safeIndex = Math.min(index, newNodes.length);
    newNodes.splice(safeIndex, 0, nodeToMove);
    return newNodes;
  }

  return mapTree(nodesWithoutSource, (node) => {
    if (node.id === targetParentId && node.type === 'folder') {
      const children = node.children || [];
      const newChildren = [...children];
      // Ensure index is within bounds
      const safeIndex = Math.min(index, newChildren.length);
      newChildren.splice(safeIndex, 0, nodeToMove);
      return {
        ...node,
        children: newChildren,
        isOpen: true,
      };
    }
    return node;
  });
};
