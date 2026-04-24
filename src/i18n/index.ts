import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import de from './locales/de.json'
import en from './locales/en.json'
import es from './locales/es.json'
import fr from './locales/fr.json'
import pt from './locales/pt.json'
import it from './locales/it.json'

i18n.use(initReactI18next).init({
  resources: {
    de: { translation: de },
    en: { translation: en },
    es: { translation: es },
    fr: { translation: fr },
    pt: { translation: pt },
    it: { translation: it },
  },
  lng: 'de',
  fallbackLng: 'de',
  supportedLngs: ['de', 'en', 'es', 'fr', 'pt', 'it'],
  interpolation: {
    escapeValue: false,
  },
})

export default i18n
