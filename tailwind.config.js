/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: [
    "./templates/**/*.{html,js}",
    "./static/**/*.{html,js}",
  ],
  theme: {
    extend: {
      colors: {
        'primary-orange': '#ff6b35',
        'primary-green': '#4ade80',
        'light-orange': '#fed7aa',
        'light-blue': '#dbeafe',
        'light-green': '#dcfce7',
        'light-yellow': '#fef3c7',
      },
      animation: {
        'gradient': 'gradient 3s ease infinite',
      },
      keyframes: {
        gradient: {
          '0%, 100%': {
            'background-size': '200% 200%',
            'background-position': 'left center'
          },
          '50%': {
            'background-size': '200% 200%',
            'background-position': 'right center'
          }
        }
      }
    },
  },
  plugins: [],
}