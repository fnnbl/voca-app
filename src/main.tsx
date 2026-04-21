import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import './i18n'
import './index.css'
import App from './App'
import StatusBar from './StatusBar'

const label = getCurrentWebviewWindow().label

if (label === 'status-bar') {
  document.documentElement.style.background = 'transparent'
  document.body.style.background = 'transparent'
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {label === 'status-bar' ? <StatusBar /> : <App />}
  </React.StrictMode>
)
