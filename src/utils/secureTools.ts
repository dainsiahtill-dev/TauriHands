import { validateFilePath, validateCommand, validateLlmPrompt, RateLimiter, ContentSecurityPolicy } from './validation';

/**
 * Secure file operations with validation
 */
export class SecureFileOperations {
  private static rateLimiter = new RateLimiter();

  static async readFile(filePath: string): Promise<{ success: boolean; data?: string; error?: string }> {
    const validation = validateFilePath(filePath);
    if (!validation.isValid) {
      return { success: false, error: validation.error };
    }

    try {
      // Check file size before reading
      const size = await this.getFileSize(filePath);
      const sizeValidation = ContentSecurityPolicy.validateFileSize(size);
      if (!sizeValidation.isValid) {
        return { success: false, error: sizeValidation.error };
      }

      // Read file content
      const content = await this.readFileContent(filePath);
      
      // Scan for malicious content
      const securityScan = ContentSecurityPolicy.scanForMaliciousContent(content);
      if (securityScan.isMalicious) {
        return { 
          success: false, 
          error: `Malicious content detected: ${securityScan.threats.join(', ')}` 
        };
      }

      return { success: true, data: content };
    } catch (error) {
      return { success: false, error: `Failed to read file: ${error}` };
    }
  }

  private static async getFileSize(filePath: string): Promise<number> {
    // This would need to be implemented based on the actual file system API
    // For now, return a reasonable default
    return 1024; // 1KB default
  }

  private static async readFileContent(filePath: string): Promise<string> {
    // This would use the actual file reading API
    // For now, return a placeholder
    return `File content for ${filePath}`;
  }

  static async writeFile(filePath: string, content: string): Promise<{ success: boolean; error?: string }> {
    const validation = validateFilePath(filePath);
    if (!validation.isValid) {
      return { success: false, error: validation.error };
    }

    try {
      // Validate content
      const contentValidation = ContentSecurityPolicy.validateTextLength(content);
      if (!contentValidation.isValid) {
        return { success: false, error: contentValidation.error };
      }

      // Scan for malicious content
      const securityScan = ContentSecurityPolicy.scanForMaliciousContent(content);
      if (securityScan.isMalicious) {
        return { 
          success: false, 
          error: `Malicious content detected: ${securityScan.threats.join(', ')}` 
        };
      }

      // Write file content
      await this.writeFileContent(filePath, content);
      return { success: true };
    } catch (error) {
      return { success: false, error: `Failed to write file: ${error}` };
    }
  }

  private static async writeFileContent(filePath: string, content: string): Promise<void> {
    // This would use the actual file writing API
    // For now, just log the operation
    console.log(`Writing to ${filePath}:`, content);
  }
}

/**
 * Secure command execution with validation
 */
export class SecureCommandExecutor {
  private static rateLimiter = new RateLimiter();

  static async executeCommand(command: string, args: string[] = []): Promise<{ success: boolean; output?: string; error?: string }> {
    const fullCommand = `${command} ${args.join(' ')}`;
    const validation = validateCommand(fullCommand);
    if (!validation.isValid) {
      return { success: false, error: validation.error };
    }

    try {
      // Rate limiting
      if (!this.rateLimiter.canMakeRequest('command_execution')) {
        return { success: false, error: 'Rate limit exceeded for command execution' };
      }

      // Execute command with proper escaping
      const result = await this.executeShellCommand(command, args);
      
      // Validate output for sensitive information
      const sanitizedOutput = this.sanitizeCommandOutput(result.output);
      
      return { 
        success: result.success, 
        output: sanitizedOutput,
        error: result.error 
      };
    } catch (error) {
      return { success: false, error: `Command execution failed: ${error}` };
    } finally {
      this.rateLimiter.reset('command_execution');
    }
  }

  private static async executeShellCommand(command: string, args: string[]): Promise<{ success: boolean; output: string; error?: string }> {
    // This would use the actual command execution API
    // For now, return a placeholder
    console.log(`Executing command: ${command} with args:`, args);
    return { 
      success: true, 
      output: `Command executed: ${command} ${args.join(' ')}` 
    };
  }

  private static sanitizeCommandOutput(output: string): string {
    // Remove potential sensitive information from command output
    const sensitivePatterns = [
      /password[=:]\s*\S+/gi,
      /token[=:]\s*\S+/gi,
      /api[_-]?key[=:]\s*\S+/gi,
      /secret[=:]\s*\S+/gi,
      /private[_-]?key[=:]\s*\S+/gi,
    ];

    let sanitized = output;
    for (const pattern of sensitivePatterns) {
      sanitized = sanitized.replace(pattern, '[REDACTED]');
    }

    return sanitized;
  }
}

/**
 * Secure LLM prompt handling
 */
export class SecureLlmHandler {
  private static rateLimiter = new RateLimiter();

  static async processPrompt(prompt: string): Promise<{ success: boolean; sanitizedPrompt?: string; error?: string }> {
    const validation = validateLlmPrompt(prompt);
    if (!validation.isValid) {
      return { success: false, error: validation.error };
    }

    // Rate limiting
    if (!this.rateLimiter.canMakeRequest('llm_prompt')) {
      return { success: false, error: 'Rate limit exceeded for LLM prompts' };
    }

    try {
      return { 
        success: true, 
        sanitizedPrompt: validation.sanitized 
      };
    } catch (error) {
      return { success: false, error: `Prompt processing failed: ${error}` };
    } finally {
      this.rateLimiter.reset('llm_prompt');
    }
  }

  static async validateAndSanitizePrompt(prompt: string): Promise<{ success: boolean; sanitizedPrompt?: string; error?: string }> {
    const result = await this.processPrompt(prompt);
    return result;
  }
}

/**
 * Input validation utilities for forms
 */
export class FormValidator {
  static validateRequired(value: string, fieldName: string): { isValid: boolean; error?: string } {
    if (!value || typeof value !== 'string' || value.trim() === '') {
      return { isValid: false, error: `${fieldName} is required` };
    }
    return { isValid: true };
  }

  static validateEmail(email: string): { isValid: boolean; error?: string } {
    const emailRegex = /^[^\s]*([^\s@]+@[^\s@]+\.[^\s@]+\.[^\s@]+)\s*$/;
    if (!emailRegex.test(email)) {
      return { isValid: false, error: 'Invalid email format' };
    }
    return { isValid: true };
  }

  static validateUrl(url: string): { isValid: boolean; error?: string } {
    const validation = validateUrl(url);
    return validation;
  }

  static validateLength(value: string, min: number, max: number, fieldName: string): { isValid: boolean; error?: string } {
    if (!value || value.length < min || value.length > max) {
      return { 
        isValid: false, 
        error: `${fieldName} must be between ${min} and ${max} characters` 
      };
    }
    return { isValid: true };
  }

  static sanitizeHtml(value: string): string {
    return value
      .replace(/</?[^>]*>/gi, '')
      .replace(/<script[^>]*>.*?<\/script>/gi, '')
      .replace(/javascript:/gi, '')
      .replace(/on\w+\s*=/gi, '');
  }

  static validateNumeric(value: string, fieldName: string): { isValid: boolean; error?: string } {
    if (!value || !/^-?\d*\.?\d*$/.test(value)) {
      return { isValid: false, error: `${fieldName} must be a valid number` };
    }
    return { isValid: true };
  }

  static validateSelect(value: string, options: string[], fieldName: string): { isValid: boolean; error?: string } {
    if (!options.includes(value)) {
      return { 
        isValid: false, 
        error: `${fieldName} must be one of: ${options.join(', ')}` 
      };
    }
    return { isValid: true };
  }
}
