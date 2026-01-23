/**
 * Input validation utilities for security
 */

// Sanitization patterns
const SANITIZATION_PATTERNS = {
  // Remove potentially dangerous characters
  dangerousChars: /[<>:"\\'`]/g,
  // Remove script tags and content
  scriptTags: /<script\b[^<]*(?:(?!<\/script>)<[^<]*)*?<\/script>/gi,
  // Remove potentially dangerous URLs
  javascriptUrls: /javascript:/gi,
  // Remove SQL injection patterns
  sqlInjection: /(\b(SELECT|INSERT|UPDATE|DELETE|DROP|CREATE|ALTER|EXEC|UNION|SCRIPT)\b)/gi,
  // Remove path traversal patterns
  pathTraversal: /\.\.[\\/]/g,
  // Remove command injection patterns
  commandInjection: /[;&|`$(){}[\]]/g,
};

/**
 * Sanitize a string input by removing potentially dangerous content
 */
export function sanitizeInput(input: string): string {
  if (typeof input !== 'string') {
    return '';
  }

  return input
    .replace(SANITIZATION_PATTERNS.dangerousChars, '')
    .replace(SANITIZATION_PATTERNS.scriptTags, '')
    .replace(SANITIZATION_PATTERNS.javascriptUrls, '')
    .replace(SANITIZATION_PATTERNS.sqlInjection, '')
    .replace(SANITIZATION_PATTERNS.pathTraversal, '')
    .replace(SANITIZATION_PATTERNS.commandInjection, '')
    .trim();
}

/**
 * Validate file path to prevent directory traversal
 */
export function validateFilePath(filePath: string): { isValid: boolean; error?: string } {
  // Check for null/undefined
  if (!filePath || typeof filePath !== 'string') {
    return { isValid: false, error: 'Invalid file path' };
  }

  // Check for path traversal attempts
  if (filePath.includes('..') || filePath.includes('~')) {
    return { isValid: false, error: 'Path traversal not allowed' };
  }

  // Check for dangerous characters
  if (/["<>:"\\'`]/.test(filePath)) {
    return { isValid: false, error: 'Invalid characters in path' };
  }

  // Normalize path
  const normalizedPath = filePath.replace(/\\/g, '/').replace(/\/+/g, '/');

  // Check for empty path segments
  const segments = normalizedPath.split('/').filter(Boolean);
  if (segments.some(segment => segment === '.' || segment === '..')) {
    return { isValid: false, error: 'Invalid path segments' };
  }

  return { isValid: true };
}

/**
 * Validate command input to prevent command injection
 */
export function validateCommand(command: string): { isValid: boolean; error?: string } {
  if (!command || typeof command !== 'string') {
    return { isValid: false, error: 'Invalid command' };
  }

  // Check for dangerous characters
  if (/[";&|`$(){}[\]]/.test(command)) {
    return { isValid: false, error: 'Invalid characters in command' };
  }

  // Check for potentially dangerous commands
  const dangerousCommands = [
    'rm -rf',
    'sudo',
    'chmod 777',
    'mkfifo',
    'nc -l',
    'curl',
    'wget',
    'eval',
    'exec',
    'system',
    'powershell',
    'cmd.exe',
    'bash',
    'sh',
  ];

  const lowerCommand = command.toLowerCase();
  if (dangerousCommands.some(dangerous => lowerCommand.includes(dangerous))) {
    return { isValid: false, error: 'Potentially dangerous command' };
  }

  return { isValid: true };
}

/**
 * Validate URL to prevent XSS attacks
 */
export function validateUrl(url: string): { isValid: boolean; error?: string } {
  if (!url || typeof url !== 'string') {
    return { isValid: false, error: 'Invalid URL' };
  }

  try {
    const parsedUrl = new URL(url);
    
    // Only allow http and https protocols
    if (!['http:', 'https:'].includes(parsedUrl.protocol)) {
      return { isValid: false, error: 'Only HTTP and HTTPS URLs are allowed' };
    }

    // Check for data URLs
    if (url.startsWith('data:')) {
      return { isValid: false, error: 'Data URLs are not allowed' };
    }

    // Check for javascript URLs
    if (SANITIZATION_PATTERNS.javascriptUrls.test(url)) {
      return { isValid: false, error: 'JavaScript URLs are not allowed' };
    }

    return { isValid: true };
  } catch (error) {
    return { isValid: false, error: 'Invalid URL format' };
  }
}

/**
 * Validate JSON input to prevent injection
 */
export function validateJson(jsonString: string): { isValid: boolean; error?: string; parsed?: any } {
  if (!jsonString || typeof jsonString !== 'string') {
    return { isValid: false, error: 'Invalid JSON string' };
  }

  try {
    const parsed = JSON.parse(jsonString);
    
    // Check for potentially dangerous content
    const jsonStr = JSON.stringify(parsed);
    if (jsonStr.includes('<script') || jsonStr.includes('javascript:')) {
      return { isValid: false, error: 'Potentially dangerous content' };
    }

    return { isValid: true, parsed };
  } catch (error) {
    return { isValid: false, error: 'Invalid JSON format' };
  }
}

/**
 * Validate LLM prompt to prevent injection
 */
export function validateLlmPrompt(prompt: string): { isValid: boolean; error?: string; sanitized?: string } {
  if (!prompt || typeof prompt !== 'string') {
    return { isValid: false, error: 'Invalid prompt' };
  }

  // Check for prompt injection patterns
  const injectionPatterns = [
    /<\s*script[^>]*>.*?<\s*\/\s*script\s*>/gi,
    /javascript:/gi,
    /data:text\/html/gi,
    /<\s*iframe[^>]*>.*?<\s*\/\s*iframe\s*>/gi,
    /<\s*embed[^>]*>.*?<\s*\/\s*embed\s*>/gi,
    /<\s*object[^>]*>.*?<\s*\/\s*object\s*>/gi,
  ];

  for (const pattern of injectionPatterns) {
    if (pattern.test(prompt)) {
      return { isValid: false, error: 'Potentially dangerous content detected' };
    }
  }

  // Check for excessive length
  if (prompt.length > 10000) {
    return { isValid: false, error: 'Prompt too long' };
  }

  return { isValid: true, sanitized: sanitizeInput(prompt) };
}

/**
 * Rate limiting for API calls
 */
export class RateLimiter {
  private requests: Map<string, number[]> = new Map();
  private windowMs: number;
  private maxRequests: number;

  constructor(windowMs: number = 60000, maxRequests: number = 10) {
    this.windowMs = windowMs;
    this.maxRequests = maxRequests;
  }

  canMakeRequest(identifier: string): boolean {
    const now = Date.now();
    const requests = this.requests.get(identifier) || [];
    
    // Remove old requests outside the window
    const validRequests = requests.filter(time => now - time < this.windowMs);
    
    if (validRequests.length >= this.maxRequests) {
      return false;
    }
    
    validRequests.push(now);
    this.requests.set(identifier, validRequests);
    return true;
  }

  reset(identifier: string): void {
    this.requests.delete(identifier);
  }
}

/**
 * Content Security Policy for user-generated content
 */
export class ContentSecurityPolicy {
  private static readonly MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB
  private static readonly MAX_TEXT_LENGTH = 50000;

  static validateFileSize(size: number): { isValid: boolean; error?: string } {
    if (size > this.MAX_FILE_SIZE) {
      return { 
        isValid: false, 
        error: `File size exceeds maximum allowed size of ${this.MAX_FILE_SIZE} bytes` 
      };
    }
    return { isValid: true };
  }

  static validateTextLength(text: string): { isValid: boolean; error?: string } {
    if (text.length > this.MAX_TEXT_LENGTH) {
      return { 
        isValid: false, 
        error: `Text length exceeds maximum allowed length of ${this.MAX_TEXT_LENGTH} characters` 
      };
    }
    return { isValid: true };
  }

  static scanForMaliciousContent(content: string): { isMalicious: boolean; threats: string[] } {
    const threats: string[] = [];
    const lowerContent = content.toLowerCase();

    // Common malicious patterns
    const maliciousPatterns = [
      { pattern: /eval\s*\(/gi, threat: 'eval() function' },
      { pattern: /document\.cookie/gi, threat: 'document.cookie access' },
      { pattern: /window\.location/gi, threat: 'window.location manipulation' },
      { pattern: /xmlhttprequest/gi, threat: 'XMLHttpRequest' },
      { pattern: /activexobject/gi, threat: 'ActiveXObject' },
      { pattern: /powershell/gi, threat: 'PowerShell command' },
      { pattern: /cmd\.exe/gi, threat: 'CMD execution' },
      { pattern: /wget|curl/gi, threat: 'Network request' },
      { pattern: /sudo|su\s/gi, threat: 'Privilege escalation' },
    ];

    for (const { pattern, threat } of maliciousPatterns) {
      if (pattern.test(lowerContent)) {
        threats.push(threat);
      }
    }

    return {
      isMalicious: threats.length > 0,
      threats
    };
  }
}
