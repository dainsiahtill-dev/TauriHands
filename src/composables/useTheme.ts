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
  name: 'Element Plus Dark',
  colors: {
    background: '#0b1220',
    surface: 'rgba(20, 30, 52, 0.85)',
    primary: '#409eff',
    secondary: '#67c23a',
    accent: '#409eff',
    text: {
      primary: '#eaf2ff',
      secondary: '#b5c4e3',
      tertiary: '#8295c6',
    },
    border: 'rgba(90, 140, 230, 0.35)',
    shadow: '0 20px 60px rgba(3, 10, 25, 0.6)',
  },
  fonts: {
    primary: '"Space Grotesk", "Manrope", sans-serif',
    secondary: '"Manrope", "Space Grotesk", sans-serif',
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
    sm: '6px',
    md: '10px',
    lg: '16px',
  },
};

const lightTheme: Theme = {
  id: 'light',
  name: 'Element Plus Light',
  colors: {
    background: '#f4f7ff',
    surface: '#ffffff',
    primary: '#409eff',
    secondary: '#67c23a',
    accent: '#409eff',
    text: {
      primary: '#1f2a44',
      secondary: '#50607f',
      tertiary: '#6f7ccf',
    },
    border: 'rgba(147, 197, 253, 0.45)',
    shadow: '0 16px 32px rgba(64, 158, 255, 0.18)',
  },
  fonts: {
    primary: '"Space Grotesk", "Manrope", sans-serif',
    secondary: '"Manrope", "Space Grotesk", sans-serif',
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
    sm: '6px',
    md: '10px',
    lg: '16px',
  },
};

const themes: Theme[] = [defaultTheme, lightTheme];
const currentThemeId = ref<string>('cyber-dark');
const currentTheme = ref<Theme>(defaultTheme);
let initialized = false;
let mediaListenerBound = false;

function hexToRgb(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed.startsWith('#')) return null;
  const hex = trimmed.replace('#', '');
  if (hex.length !== 6) return null;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return null;
  return `${r}, ${g}, ${b}`;
}

export function useTheme() {
  const isDark = computed(() => currentTheme.value.id === 'cyber-dark');
  const isLight = computed(() => currentTheme.value.id === 'light');

  const applyTheme = (theme: Theme) => {
    const root = document.documentElement;
    const isDarkTheme = theme.id === 'cyber-dark';
    const bgRgb = hexToRgb(theme.colors.background);
    const accentRgb = hexToRgb(theme.colors.accent);
    const accent2Rgb = hexToRgb(theme.colors.secondary);
    const accent3Rgb = hexToRgb(theme.colors.primary);
    const textSecondaryRgb = hexToRgb(theme.colors.text.secondary);
    const textTertiaryRgb = hexToRgb(theme.colors.text.tertiary);

    // Apply CSS custom properties
    root.style.setProperty('--bg', theme.colors.background);
    if (bgRgb) {
      root.style.setProperty('--bg-rgb', bgRgb);
    }
    root.style.setProperty('--surface', theme.colors.surface);
    root.style.setProperty('--primary', theme.colors.primary);
    root.style.setProperty('--secondary', theme.colors.secondary);
    root.style.setProperty('--accent', theme.colors.accent);
    root.style.setProperty('--accent-2', theme.colors.secondary);
    root.style.setProperty('--accent-3', theme.colors.primary);
    if (accentRgb) {
      root.style.setProperty('--accent-rgb', accentRgb);
    }
    if (accent2Rgb) {
      root.style.setProperty('--accent-2-rgb', accent2Rgb);
    }
    if (accent3Rgb) {
      root.style.setProperty('--accent-3-rgb', accent3Rgb);
    }
    root.style.setProperty('--text-primary', theme.colors.text.primary);
    root.style.setProperty('--text-secondary', theme.colors.text.secondary);
    root.style.setProperty('--text-tertiary', theme.colors.text.tertiary);
    if (textSecondaryRgb) {
      root.style.setProperty('--text-secondary-rgb', textSecondaryRgb);
    }
    if (textTertiaryRgb) {
      root.style.setProperty('--text-tertiary-rgb', textTertiaryRgb);
      root.style.setProperty('--line-rgb', textTertiaryRgb);
    }
    root.style.setProperty('--border', theme.colors.border);
    root.style.setProperty('--shadow', theme.colors.shadow);

    // Terminal surface tint
    root.style.setProperty(
      '--terminal-bg',
      isDarkTheme ? 'rgba(12, 20, 36, 0.96)' : 'rgba(18, 28, 60, 0.95)',
    );
    root.style.setProperty(
      '--terminal-border',
      isDarkTheme ? 'rgba(90, 140, 230, 0.35)' : 'rgba(64, 158, 255, 0.28)',
    );

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

    // Set theme attributes + Element Plus dark trigger
    root.setAttribute('data-theme', theme.id);
    root.setAttribute('data-theme-mode', isDarkTheme ? 'dark' : 'light');
    root.classList.toggle('dark', isDarkTheme);
    root.style.colorScheme = isDarkTheme ? 'dark' : 'light';
  };

  const saveThemePreference = (themeId: string) => {
    try {
      localStorage.setItem('theme-preference', themeId);
    } catch (error) {
      console.warn('Failed to save theme preference:', error);
    }
  };

  const loadThemePreference = (): string | null => {
    try {
      return localStorage.getItem('theme-preference');
    } catch (error) {
      console.warn('Failed to load theme preference:', error);
      return null;
    }
  };

  const getSystemTheme = (): string => {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'cyber-dark';
    }
    return 'light';
  };

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

  const initializeTheme = () => {
    const savedTheme = loadThemePreference();
    const systemTheme = getSystemTheme();
    const themeId = savedTheme || systemTheme;
    setTheme(themeId);
  };

  if (!initialized) {
    initializeTheme();
    initialized = true;
  }

  if (window.matchMedia && !mediaListenerBound) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      const systemTheme = e.matches ? 'cyber-dark' : 'light';
      const savedTheme = loadThemePreference();
      if (!savedTheme) {
        setTheme(systemTheme);
      }
    });
    mediaListenerBound = true;
  }

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
