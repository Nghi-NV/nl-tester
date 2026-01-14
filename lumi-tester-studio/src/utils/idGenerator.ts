/**
 * Generate a simple random ID
 */
export const generateId = (): string => {
  return Math.random().toString(36).substring(2, 9);
};

/**
 * Generate a run ID (longer for uniqueness)
 */
export const generateRunId = (): string => {
  return Math.random().toString(36).substring(2, 15);
};
