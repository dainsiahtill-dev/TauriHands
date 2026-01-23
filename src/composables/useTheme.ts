import { ref, computed, watch } from 'vue';

export interface Theme {
  id: string;
  name: string;
  colors: {
    background: string;
    surface: string;
    primary: string;
    secondary: string;
    accent: string;
    text: {
      primary: string;
      secondary: string;
      tertiary: string;
    };
    border: string;
    shadow: string;
  };
  fonts: {
    primary: string;
    secondary: string;
    mono: string;
  };
  spacing: {
    xs: string;
    sm: string;
    md: string;
    lg: string;
    xl: string;
  };
  borderRadius: {
    sm: string;
    md: string;
    lg: string;
  };
}

const defaultTheme: Theme = {
  id: 'cyber-dark',
  name: 'Cyber Dark',
  colors: {
    background: '#020408',
    surface: 'rgba(2, 10, 20, 0.85)',
    primary: '#00f3ff',
    secondary: '#0088ff',
    accent: '#00c2ff',
    text: {
      primary: '#d2f6ff',
      secondary: '#8ecbe0',
      tertiary: '#3f6b85',
    },
    border: 'rgba(0, 243, 255, 0.3)',
    shadow: '0 32px 70px rgba(2, 6, 16, 0.75)',
  },
  fonts: {
    primary: '"Oxanium", "JetBrains Mono", sans-serif',
    secondary: '"JetBrains Mono", "Courier New", monospace',
    mono: '"JetBrains Mono", "Courier New", monospace',
  },
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
  },
  borderRadius: {
    sm: '4px',
    md: '8px',
    lg: '12px',
  },
};

const lightTheme: Theme = {
  id: 'light',
  name: 'Light',
  colors: {
    background: '#ffffff',
    surface: '#f8fafc',
    primary: '#3b82f6',
    secondary: '#6366f1',
    accent: '#06b6d4',
    text: {
      primary: '#1e293b',
      secondary: '#475569',
      tertiary: '#94a3b8',
    },
    border: 'rgba(0, 0, 0, 0.1)',
    shadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
  },
  fonts: {
    primary: '"Inter", "Helvetica Neue", sans-serif',
    secondary: '"JetBrains Mono", "Courier New", monospace',
    mono: '"JetBrains Mono", "Courier New", monospace',
  },
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
  },
  borderRadius: {
    sm: '4px',
    md: '8px',
    lg: '12px',
  },
};

const themes: Theme[] = [defaultTheme, lightTheme];

export function useTheme() {
  const currentThemeId = ref<string>('cyber-dark');
  const currentTheme = ref<Theme>(defaultTheme);

  const isDark = computed(() => currentTheme.value.id === 'cyber-dark');
  const isLight = computed(() => currentTheme.value.id === 'light');

  const setTheme = (themeId: string) => {
    const theme = themes.find(t => t.id === themeId);
    if (theme) {
      currentThemeId.value = themeId;
      currentTheme.value = theme;
      applyTheme(theme);
      saveThemePreference(themeId);
    }
  };

  const toggleTheme = () => {
    const nextTheme = isDark.value ? 'light' : 'cyber-dark';
    setTheme(nextTheme);
  };

  const applyTheme = (theme: Theme) => {
    const root = document.documentElement;
    
    // Apply CSS custom properties
    root.style.setProperty('--bg', theme.colors.background);
    root.style.setProperty('--surface', theme.colors.surface);
    root.style.setProperty('--primary', theme.colors.primary);
    root.style.setProperty('--secondary', theme.colors.secondary);
    root.style.setProperty('--accent', theme.colors.accent);
    root.style.setProperty('--text-primary', theme.colors.text.primary);
    root.style.setProperty('--text-secondary', theme.colors.text.secondary);
    root.style.setProperty('--text-tertiary', theme.colors.text.tertiary);
    root.style.setProperty('--border', theme.colors.border);
    root.style.setProperty('--shadow', theme.colors.shadow);
    
    // Apply fonts
    root.style.setProperty('--font-primary', theme.fonts.primary);
    root.style.setProperty('--font-secondary', theme.fonts.secondary);
    root.style.setProperty('--font-mono', theme.fonts.mono);
    
    // Apply spacing
    Object.entries(theme.spacing).forEach(([key, value]) => {
      root.style.setProperty(`--spacing-${key}`, value);
    });
    
    // Apply border radius
    Object.entries(theme.borderRadius).forEach(([key, value]) => {
      root.style.setProperty(`--border-radius-${key}`, value);
    });
    
    // Set theme attributes
    root.setAttribute('data-theme', theme.id);
    root.setAttribute('data-theme-mode', isDark.value ? 'dark' : 'light');
  };

  const saveThemePreference = (themeId: string) => {
    try {
      localStorage.setItem('theme-preference', themeId);
    } catch (error) {
      console.warn('Failed to save theme preference:', error);
    }
  };

  const loadThemePreference = (): string => {
    try {
      return localStorage.getItem('theme-preference') || 'cyber-dark';
    } catch (error) {
      console.warn('Failed to load theme preference:', error);
      return 'cyber-dark';
    }
  };

  const getSystemTheme = (): string => {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'cyber-dark';
    }
    return 'light';
  };

  const initializeTheme = () => {
    const savedTheme = loadThemePreference();
    const systemTheme = getSystemTheme();
    
    // Use saved theme if available, otherwise use system preference
    const themeId = savedTheme || systemTheme;
    setTheme(themeId);
  };

  // Watch for system theme changes
  if (window.matchMedia) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      const systemTheme = e.matches ? 'cyber-dark' : 'light';
      const savedTheme = loadThemePreference();
      
      // Only auto-switch if user hasn't explicitly set a preference
      if (!savedTheme) {
        setTheme(systemTheme);
      }
    });
  }

  // Initialize theme on mount
  initializeTheme();

  return {
    currentTheme: computed(() => currentTheme.value),
    currentThemeId: computed(() => currentThemeId.value),
    isDark,
    isLight,
    availableThemes: themes,
    setTheme,
    toggleTheme,
  };
}
